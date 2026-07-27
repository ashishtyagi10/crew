# Provider Auth SP1 — Credential Store + Key Popup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user supply a provider API key from inside crew — accepting a dimmed model row opens a masked field, and the key takes effect on the next message with no restart.

**Architecture:** A JSON credential store in `crew-plugin` (`<config_dir>/crew/credentials.json`, mode 0600) is read by the broker at startup, ahead of the login-shell hydration but behind any variable exported into the process. The app gets a modal masked popup that writes to that store. The broker rebuilds its registry per request, so no restart is needed anywhere.

**Tech Stack:** Rust, `crew-plugin` (broker), `crew-app` (winit/wgpu GUI), `serde_json`, `dirs`, `anyhow` — all already dependencies of both crates.

**Spec:** `docs/superpowers/specs/2026-07-26-provider-auth-key-popup-design.md`

## Global Constraints

- **NEVER run `cargo build --release` or `cargo clean`** — disk is tight on this machine. `cargo test` / `cargo clippy` (dev profile) only.
- **No new dependencies.** `serde`, `serde_json`, `anyhow`, `dirs` are already in both crates.
- **Never log, print, echo, export or persist a key's VALUE anywhere but the credentials file.** Not in a `tracing`/`eprintln` line, not in the chat transcript, not in the session log, not in `/dump`, not in an error message. Error messages name the variable, never the value.
- **Tests must not mutate process-global environment or write to the real config directory.** Both are shared by ~1500 parallel tests in one binary. Every store test takes an explicit path; every precedence test exercises a pure helper.
- **Exact variable set:** `DASHSCOPE_API_KEY`, `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`. Exact provider names: `dashscope`, `openrouter`, `anthropic` (these are `CREW_PROVIDER`'s accepted values — `discover.rs::pick_provider` matches them lowercased).
- **File permissions are `#[cfg(unix)]`** — dir `0700`, file `0600`. crew releases darwin + linux.
- Keep source files under ~200 lines; tests in a sibling `<name>_tests.rs` via `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.
- `cargo clippy --workspace --all-targets -- -D warnings` must be green, with no `#[allow(...)]` added.
- Never commit keys — this repo is public. Test fixtures use obvious fakes like `"sk-test-not-a-real-key"`.

---

### Task 1: The credential store

**Files:**
- Create: `crates/crew-plugin/src/credentials.rs`
- Create: `crates/crew-plugin/src/credentials_tests.rs`
- Modify: `crates/crew-plugin/src/lib.rs` (declare and export the module)

**Interfaces:**
- Consumes: nothing.
- Produces, all `pub` from `crew_plugin::credentials`:
  - `VARS: [&str; 3]`
  - `provider_for(var: &str) -> Option<&'static str>`
  - `Store { provider: Option<String>, keys: BTreeMap<String, String> }` (derives `Default, Debug, Clone, PartialEq, Serialize, Deserialize`)
  - `path() -> Option<PathBuf>`
  - `load() -> Store` / `load_from(path: &Path) -> Store`
  - `save_key(var, value, provider) -> anyhow::Result<()>` / `save_key_at(path, var, value, provider) -> anyhow::Result<()>`
  - Task 2 calls `load()` and `VARS`; Task 5 calls `save_key` and `provider_for`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-plugin/src/credentials_tests.rs`:

```rust
use super::*;

/// A unique scratch path per test — the store is a file, and the real config
/// directory must never be touched by a test run.
fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-cred-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("credentials.json")
}

#[test]
fn save_then_load_round_trips_the_key_and_pin() {
    let p = scratch("roundtrip");
    save_key_at(&p, "ANTHROPIC_API_KEY", "sk-test-not-a-real-key", Some("anthropic")).unwrap();
    let s = load_from(&p);
    assert_eq!(s.keys.get("ANTHROPIC_API_KEY").map(String::as_str), Some("sk-test-not-a-real-key"));
    assert_eq!(s.provider.as_deref(), Some("anthropic"));
}

#[test]
fn a_second_key_joins_the_first_and_moves_the_pin() {
    let p = scratch("second");
    save_key_at(&p, "ANTHROPIC_API_KEY", "sk-a", Some("anthropic")).unwrap();
    save_key_at(&p, "DASHSCOPE_API_KEY", "sk-d", Some("dashscope")).unwrap();
    let s = load_from(&p);
    assert_eq!(s.keys.len(), 2, "the first key must survive the second save");
    assert_eq!(s.provider.as_deref(), Some("dashscope"));
}

#[test]
fn an_empty_value_removes_rather_than_stores_a_blank() {
    // A blank ANTHROPIC_API_KEY is exactly the trap that outranks a valid
    // OAuth profile — the store must never hold one.
    let p = scratch("empty");
    save_key_at(&p, "ANTHROPIC_API_KEY", "sk-a", None).unwrap();
    save_key_at(&p, "ANTHROPIC_API_KEY", "", None).unwrap();
    assert!(!load_from(&p).keys.contains_key("ANTHROPIC_API_KEY"));
}

#[test]
fn an_unknown_variable_is_refused() {
    let p = scratch("unknown");
    let err = save_key_at(&p, "AWS_SECRET_ACCESS_KEY", "x", None).unwrap_err();
    assert!(err.to_string().contains("AWS_SECRET_ACCESS_KEY"));
    assert!(!p.exists(), "a refused write must not create the file");
}

#[test]
fn a_missing_or_malformed_file_loads_as_default() {
    let p = scratch("malformed");
    assert_eq!(load_from(&p), Store::default(), "missing file");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, "{ this is not json").unwrap();
    assert_eq!(load_from(&p), Store::default(), "malformed file must not break startup");
}

#[test]
fn the_write_is_atomic_and_leaves_no_temp_file() {
    let p = scratch("atomic");
    save_key_at(&p, "OPENROUTER_API_KEY", "sk-o", None).unwrap();
    let leftovers: Vec<_> = std::fs::read_dir(p.parent().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "temp file left behind: {leftovers:?}");
}

#[cfg(unix)]
#[test]
fn the_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let p = scratch("perms");
    save_key_at(&p, "OPENROUTER_API_KEY", "sk-o", None).unwrap();
    let file = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
    let dir = std::fs::metadata(p.parent().unwrap()).unwrap().permissions().mode() & 0o777;
    assert_eq!(file, 0o600, "credentials file must be owner-only");
    assert_eq!(dir, 0o700, "credentials dir must be owner-only");
}

#[test]
fn provider_for_maps_every_var_and_nothing_else() {
    assert_eq!(provider_for("DASHSCOPE_API_KEY"), Some("dashscope"));
    assert_eq!(provider_for("OPENROUTER_API_KEY"), Some("openrouter"));
    assert_eq!(provider_for("ANTHROPIC_API_KEY"), Some("anthropic"));
    assert_eq!(provider_for("AWS_SECRET_ACCESS_KEY"), None);
    for v in VARS {
        assert!(provider_for(v).is_some(), "{v} has no provider mapping");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-plugin credentials`
Expected: compile error — the module does not exist.

- [ ] **Step 3: Write the module**

Create `crates/crew-plugin/src/credentials.rs`:

```rust
//! Provider credentials supplied from inside crew, rather than from the
//! environment. Stored as JSON next to `config.toml` — deliberately NOT inside
//! `CrewConfig`, which is user-visible, hand-edited and safe to paste around;
//! a key in there would leak the first time someone shared their config.
//!
//! Lives in `crew-plugin` because both consumers reach it here: the broker IS
//! this crate, and `crew-app` already depends on it.
//!
//! Not the macOS Keychain: crew ships Linux binaries too, so Keychain would
//! mean two code paths and a platform-specific failure mode for a v1. An
//! owner-only file is what `gh` and `aws` do. Keychain stays open as an upgrade.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The only variables this store will hold, so a key typed into the UI can
/// never name an arbitrary environment variable.
pub const VARS: [&str; 3] = [
    "DASHSCOPE_API_KEY",
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
];

/// The provider a variable authenticates, spelled as `CREW_PROVIDER` and
/// `pick_provider` spell it.
pub fn provider_for(var: &str) -> Option<&'static str> {
    match var {
        "DASHSCOPE_API_KEY" => Some("dashscope"),
        "OPENROUTER_API_KEY" => Some("openrouter"),
        "ANTHROPIC_API_KEY" => Some("anthropic"),
        _ => None,
    }
}

/// The on-disk shape. `provider` is the pin written when a key is saved: with
/// `pick_provider`'s fixed DashScope → OpenRouter → Anthropic order, supplying
/// an Anthropic key while a DashScope key exists would otherwise change
/// nothing the user can see.
#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Store {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
}

/// `<config_dir>/crew/credentials.json`, a sibling of `config.toml`.
pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("credentials.json"))
}

/// The stored credentials, or an empty store. EVERY failure — no config dir,
/// no file, unreadable, malformed JSON — reads as empty: a broken credentials
/// file must never stop crew from starting.
pub fn load() -> Store {
    path().map(|p| load_from(&p)).unwrap_or_default()
}

/// [`load`] from an explicit path (the testable half).
pub fn load_from(path: &Path) -> Store {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Store `value` for `var`, optionally moving the provider pin. An empty
/// `value` REMOVES the key rather than storing a blank — an empty
/// `ANTHROPIC_API_KEY` still outranks a valid OAuth profile, so a blank is
/// worse than nothing.
///
/// Never logs `value`.
pub fn save_key(var: &str, value: &str, provider: Option<&str>) -> anyhow::Result<()> {
    let path = path().ok_or_else(|| anyhow::anyhow!("no config directory to store credentials in"))?;
    save_key_at(&path, var, value, provider)
}

/// [`save_key`] at an explicit path (the testable half).
pub fn save_key_at(
    path: &Path,
    var: &str,
    value: &str,
    provider: Option<&str>,
) -> anyhow::Result<()> {
    if !VARS.contains(&var) {
        anyhow::bail!("{var} is not a provider key crew stores");
    }
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("credentials path has no parent directory"))?;
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let mut store = load_from(path);
    if value.is_empty() {
        store.keys.remove(var);
    } else {
        store.keys.insert(var.to_string(), value.to_string());
    }
    if let Some(p) = provider {
        store.provider = Some(p.to_string());
    }
    write_atomic(path, &serde_json::to_vec_pretty(&store)?)
}

/// Write via a same-directory temp file created 0600 BEFORE any content lands
/// in it, then rename over the target. There is never a moment when the
/// secret exists in a world-readable file, and a crash leaves either the old
/// file or the temp — never a truncated one.
fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("json.tmp");
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
```

Declare it in `crates/crew-plugin/src/lib.rs` alongside the other module declarations:

```rust
pub mod credentials;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p crew-plugin credentials`
Expected: PASS, all nine tests.

- [ ] **Step 5: Lint and commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings` — expect clean.

```bash
git add crates/crew-plugin/src/credentials.rs crates/crew-plugin/src/credentials_tests.rs \
        crates/crew-plugin/src/lib.rs
git commit -m "feat(broker): owner-only credential store for provider keys"
```

---

### Task 2: The broker reads the store

**Files:**
- Modify: `crates/crew-plugin/src/broker/shellenv.rs` (credentials pass before the shell pass)
- Modify: `crates/crew-plugin/src/broker/discover.rs` (`forced_provider`, two call sites)
- Modify: `crates/crew-plugin/src/broker/doctor.rs:135` and `crates/crew-plugin/src/broker/stdio.rs:305` (use `forced_provider`)
- Modify: `crates/crew-plugin/src/broker/shellenv_tests.rs` (new tests)

**Interfaces:**
- Consumes: `crate::credentials::{load, VARS}` (Task 1).
- Produces: `broker::discover::forced_provider() -> Option<String>` — `pub(crate)`, the single answer to "which provider is pinned", used by every reader that previously called `std::env::var("CREW_PROVIDER")`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-plugin/src/broker/shellenv_tests.rs`:

```rust
#[test]
fn credentials_fill_only_what_the_process_env_lacks() {
    // Pure helper: no process env mutation, because ~1500 tests share it.
    let store = crate::credentials::Store {
        provider: Some("anthropic".into()),
        keys: [
            ("ANTHROPIC_API_KEY".to_string(), "sk-from-store".to_string()),
            ("DASHSCOPE_API_KEY".to_string(), "sk-also-store".to_string()),
            ("OPENROUTER_API_KEY".to_string(), String::new()),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    // ANTHROPIC is already exported non-empty; DASHSCOPE is empty in the env.
    let current = |k: &str| match k {
        "ANTHROPIC_API_KEY" => Some("sk-from-env".to_string()),
        "DASHSCOPE_API_KEY" => Some(String::new()),
        _ => None,
    };
    let imports = super::credential_imports(&store, current);
    assert_eq!(
        imports,
        vec![("DASHSCOPE_API_KEY".to_string(), "sk-also-store".to_string())],
        "an exported var wins; an empty env var is filled; an empty stored value is skipped"
    );
}

#[test]
fn credentials_never_import_a_variable_outside_vars() {
    let store = crate::credentials::Store {
        keys: [("AWS_SECRET_ACCESS_KEY".to_string(), "nope".to_string())]
            .into_iter()
            .collect(),
        ..Default::default()
    };
    assert!(super::credential_imports(&store, |_| None).is_empty());
}
```

Append to `crates/crew-plugin/src/broker/discover_tests.rs` (create the file and attach it with `#[cfg(test)] #[path = "discover_tests.rs"] mod tests;` at the bottom of `discover.rs` if it does not already exist):

```rust
use super::*;

#[test]
fn the_env_pin_outranks_the_stored_pin() {
    assert_eq!(
        resolve_forced(Some("openrouter".to_string()), Some("anthropic".to_string())).as_deref(),
        Some("openrouter"),
        "an explicit CREW_PROVIDER=… crew must never be overridden by a stored pin"
    );
}

#[test]
fn the_stored_pin_applies_when_the_env_is_unset_or_blank() {
    assert_eq!(
        resolve_forced(None, Some("anthropic".to_string())).as_deref(),
        Some("anthropic")
    );
    assert_eq!(
        resolve_forced(Some(String::new()), Some("anthropic".to_string())).as_deref(),
        Some("anthropic"),
        "a blank CREW_PROVIDER is not a pin"
    );
}

#[test]
fn no_pin_anywhere_leaves_auto_discovery_alone() {
    assert_eq!(resolve_forced(None, None), None);
    assert_eq!(resolve_forced(None, Some(String::new())), None);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-plugin shellenv && cargo test -p crew-plugin discover`
Expected: compile error — `credential_imports` and `resolve_forced` do not exist.

- [ ] **Step 3: Add the credentials pass to `shellenv.rs`**

Add above `hydrate`:

```rust
/// Which stored credentials should be imported, given a reader of the current
/// process environment. Split out from [`hydrate`] so the precedence rule is
/// testable without mutating process-global state.
///
/// A variable already exported non-empty into this process WINS — that is the
/// most deliberate signal a user can send. Everything else the store holds is
/// imported, which puts it AHEAD of the login-shell pass below: a key typed
/// into crew beats a stale value in a shell rc file, or the user would type a
/// key, see nothing change, and have no way to find out why.
fn credential_imports(
    store: &crate::credentials::Store,
    current: impl Fn(&str) -> Option<String>,
) -> Vec<(String, String)> {
    store
        .keys
        .iter()
        .filter(|(k, v)| {
            !v.is_empty()
                && crate::credentials::VARS.contains(&k.as_str())
                && current(k).is_none_or(|cur| cur.is_empty())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}
```

If the workspace's Rust version rejects `is_none_or`, use `.map_or(true, |cur| cur.is_empty())` instead — the existing `missing` closure in `hydrate` uses that form, so match whichever the file already compiles with.

Then, inside `hydrate`, **after** the `CREW_SHELL_ENV` early return and **before** the shell probe:

```rust
    // Deliberately after the CREW_SHELL_ENV=0 gate: that switch exists so the
    // e2e harness never inherits a developer's real keys, and stored
    // credentials are exactly as much "the developer's real keys" as their
    // shell env is.
    for (k, v) in credential_imports(&crate::credentials::load(), |k| std::env::var(k).ok()) {
        std::env::set_var(k, v);
    }
```

- [ ] **Step 4: Add `forced_provider` to `discover.rs`**

Add near `pick_provider`:

```rust
/// Which provider is pinned: `CREW_PROVIDER` if it names one, else the pin the
/// credential store recorded when a key was saved. Env wins, so an explicit
/// `CREW_PROVIDER=… crew` is never overridden by something crew stored itself.
pub(crate) fn forced_provider() -> Option<String> {
    resolve_forced(
        std::env::var("CREW_PROVIDER").ok(),
        crate::credentials::load().provider,
    )
}

/// The pure half of [`forced_provider`], so the precedence is testable without
/// the process environment or the real config directory.
fn resolve_forced(env: Option<String>, stored: Option<String>) -> Option<String> {
    env.filter(|v| !v.is_empty())
        .or_else(|| stored.filter(|v| !v.is_empty()))
}
```

Replace all four readers with it:

- `discover.rs:110` — `pick_provider(std::env::var("CREW_PROVIDER").ok().as_deref(), …)` becomes `pick_provider(forced_provider().as_deref(), …)`
- `discover.rs:128` — `let force = std::env::var("CREW_PROVIDER").ok();` becomes `let force = forced_provider();`
- `doctor.rs:135` — same substitution, calling `super::discover::forced_provider()`
- `stdio.rs:305` — same substitution, calling `super::discover::forced_provider()`

All four must change together: a pin that applies to the roster but not to `doctor`'s report would make `/doctor` describe a provider the broker is not using.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p crew-plugin`
Expected: PASS. If an existing test asserted a `CREW_PROVIDER`-driven behaviour and now sees a stored pin, that means the test is reading the real config directory — report it rather than working around it; the store must never be read from a test.

- [ ] **Step 6: Lint and commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings` — expect clean.

```bash
git add crates/crew-plugin/src/broker
git commit -m "feat(broker): import stored credentials and honour the stored provider pin"
```

---

### Task 3: Carry the needed key to the accept path

App-side plumbing only — no UI yet. After this task, accepting a dimmed row produces a distinct outcome carrying the variable name.

**Files:**
- Modify: `crates/crew-app/src/modelroute.rs` (add `Route::needs_key`)
- Modify: `crates/crew-app/src/suggest.rs` (add `MenuItem.needs`)
- Modify: `crates/crew-app/src/modelpick.rs` (fill `needs` in `model_row`; `None` in `default_row`, `header_row`)
- Modify: `crates/crew-app/src/chatpaletteitems.rs` (two literals), `crates/crew-app/src/suggest.rs` (two literals), `crates/crew-app/src/route.rs` (one literal), `crates/crew-app/src/render.rs` (one literal) — each gains `needs: None`
- Modify: `crates/crew-app/src/chatpalette.rs` (add `PaletteKey::NeedsKey`)
- Modify: `crates/crew-app/src/chat.rs` (the `match` at :362 gains the arm)
- Modify: `crates/crew-app/src/modelroute_tests.rs`, `crates/crew-app/src/chatpalette_tests.rs` (new tests)

**Interfaces:**
- Consumes: `crew_plugin::credentials::VARS` (Task 1).
- Produces:
  - `Route::needs_key(&self) -> Option<&'static str>`
  - `suggest::MenuItem.needs: Option<String>`
  - `chatpalette::PaletteKey::NeedsKey(String)`
  - Task 5 replaces this task's placeholder arm in `chat.rs` with the popup-opening one.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/modelroute_tests.rs`:

```rust
#[test]
fn needs_key_names_only_real_variables() {
    assert_eq!(
        Route::Missing("ANTHROPIC_API_KEY").needs_key(),
        Some("ANTHROPIC_API_KEY")
    );
    // `route_for` also produces this human phrase for an OpenRouter-unservable
    // model. It names no variable, and must never open a key prompt.
    assert_eq!(Route::Missing("a model OpenRouter serves").needs_key(), None);
    assert_eq!(Route::Direct("anthropic").needs_key(), None);
    assert_eq!(Route::ViaOpenRouter.needs_key(), None);
    assert_eq!(Route::Unknown.needs_key(), None);
    assert_eq!(Route::Mock.needs_key(), None);
}
```

Append to `crates/crew-app/src/chatpalette_tests.rs`:

```rust
#[test]
fn accepting_a_row_that_needs_a_key_asks_for_it_instead_of_running() {
    let mut input = "/model claude".to_string();
    let before = input.clone();
    let mut palette = Some(PaletteState {
        kind: Kind::Model,
        items: vec![MenuItem {
            label: "Claude Opus".into(),
            desc: "needs ANTHROPIC_API_KEY".into(),
            fill: "claude-opus-5".into(),
            submit: true,
            header: false,
            dim: true,
            needs: Some("ANTHROPIC_API_KEY".into()),
        }],
        sel: 0,
        touched: true,
    });
    match popup_key(&mut palette, &mut input, &ChatInput::Enter) {
        PaletteKey::NeedsKey(var) => assert_eq!(var, "ANTHROPIC_API_KEY"),
        _ => panic!("a keyless row must not run"),
    }
    assert_eq!(input, before, "the model must not be chosen until it can run");
    assert!(palette.is_none(), "the palette closes to make room for the prompt");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app modelroute && cargo test -p crew-app chatpalette`
Expected: compile errors — no `needs_key`, no `needs` field, no `NeedsKey` variant.

- [ ] **Step 3: Add `Route::needs_key`**

In `crates/crew-app/src/modelroute.rs`, inside `impl Route`:

```rust
    /// The environment variable this row is blocked on, when that is a key
    /// crew can actually store. `Missing` also carries human phrases — see
    /// `route_for`'s `Missing("a model OpenRouter serves")` — which name no
    /// variable and must not produce a key prompt for a nonsense name.
    pub(crate) fn needs_key(&self) -> Option<&'static str> {
        match self {
            Self::Missing(k) => crew_plugin::credentials::VARS.contains(k).then_some(*k),
            _ => None,
        }
    }
```

- [ ] **Step 4: Add the `needs` field and fill it**

In `crates/crew-app/src/suggest.rs`, add to `MenuItem`:

```rust
    /// The provider key this row is blocked on (`Route::needs_key`). Accepting
    /// such a row prompts for the key instead of choosing a model that cannot
    /// run. `None` everywhere outside the model picker.
    pub needs: Option<String>,
```

In `crates/crew-app/src/modelpick.rs`, `model_row` gains:

```rust
        needs: route.needs_key().map(str::to_string),
```

Add `needs: None` to every other `MenuItem` literal — there are six: `modelpick.rs:141` (`default_row`), `modelpick.rs:152` (`header_row`), `chatpaletteitems.rs:11`, `chatpaletteitems.rs:34`, `suggest.rs:74`, `suggest.rs:88`, `route.rs:59`, `render.rs:157`. Add it to test literals too as the compiler finds them.

- [ ] **Step 5: Add the palette outcome**

In `crates/crew-app/src/chatpalette.rs`, add to `PaletteKey`:

```rust
    /// The accepted row can't run until a provider key exists; the payload is
    /// the variable it needs. The palette is closed and `input` is UNCHANGED —
    /// the model is not chosen until it can actually be served.
    NeedsKey(String),
```

and in `popup_key`'s `Complete | Enter` arm, before the existing `submit` logic:

```rust
            let mut submit = false;
            if let Some(item) = p.items.get(p.sel) {
                if let Some(var) = item.needs.clone() {
                    *palette = None;
                    return PaletteKey::NeedsKey(var);
                }
                submit = item.submit && matches!(key, ChatInput::Enter);
                *input = accept(input, p.kind, &item.fill);
            }
```

In `crates/crew-app/src/chat.rs`, the `match` at :362 gains a placeholder arm so it stays exhaustive — Task 5 replaces its body:

```rust
            // Task 5 opens the key popup here.
            crate::chatpalette::PaletteKey::NeedsKey(_) => return None,
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS.

- [ ] **Step 7: Lint and commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings` — expect clean.

```bash
git add crates/crew-app/src
git commit -m "feat(chat): route a keyless model row to a key prompt instead of running it"
```

---

### Task 4: The masked key popup

A self-contained module: state, key handling, rendering. Not yet wired into the pane — Task 5 does that.

**Files:**
- Create: `crates/crew-app/src/keyentry.rs`
- Create: `crates/crew-app/src/keyentry_tests.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod keyentry;`)

**Interfaces:**
- Consumes: `crate::chatkeys::ChatInput`, `crate::boxdraw::titled_card`, `crew_render::CellView`.
- Produces:
  - `keyentry::KeyEntry { var: String }` with `new(var: String)`, `key(&mut self, &ChatInput) -> KeyOutcome`, `card(&self, cols: u16) -> Vec<CellView>`
  - `keyentry::KeyOutcome { Consumed, Cancelled, Submit(String) }`
  - `keyentry::ROWS: u16` — the popup's total height, for `render.rs`'s layout in Task 5.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/keyentry_tests.rs`:

```rust
use super::*;
use crate::chatkeys::ChatInput;

fn typed(e: &mut KeyEntry, s: &str) {
    for c in s.chars() {
        assert!(matches!(e.key(&ChatInput::Char(c)), KeyOutcome::Consumed));
    }
}

#[test]
fn typing_then_enter_submits_exactly_what_was_typed() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, "sk-test-not-a-real-key");
    match e.key(&ChatInput::Enter) {
        KeyOutcome::Submit(v) => assert_eq!(v, "sk-test-not-a-real-key"),
        _ => panic!("expected a submit"),
    }
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    // Pasting a key commonly drags a trailing newline or space with it.
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, "  sk-padded  ");
    match e.key(&ChatInput::Enter) {
        KeyOutcome::Submit(v) => assert_eq!(v, "sk-padded"),
        _ => panic!("expected a submit"),
    }
}

#[test]
fn backspace_deletes_and_escape_cancels() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, "abc");
    assert!(matches!(e.key(&ChatInput::Backspace), KeyOutcome::Consumed));
    match e.key(&ChatInput::Enter) {
        KeyOutcome::Submit(v) => assert_eq!(v, "ab"),
        _ => panic!("expected a submit"),
    }
    let mut e2 = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e2, "abc");
    assert!(matches!(e2.key(&ChatInput::Close), KeyOutcome::Cancelled));
}

#[test]
fn enter_on_an_empty_buffer_does_not_submit() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    assert!(matches!(e.key(&ChatInput::Enter), KeyOutcome::Consumed));
    typed(&mut e, "   ");
    assert!(matches!(e.key(&ChatInput::Enter), KeyOutcome::Consumed));
}

#[test]
fn the_popup_is_modal_and_swallows_other_keys() {
    // Arrows and Tab must not leak to the pane underneath while a secret is
    // half-typed.
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    for k in [ChatInput::Up, ChatInput::Down, ChatInput::Complete, ChatInput::Newline] {
        assert!(matches!(e.key(&k), KeyOutcome::Consumed), "{k:?} leaked");
    }
}

#[test]
fn the_card_masks_every_character_and_never_draws_the_secret() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    let secret = "sk-supersecret";
    typed(&mut e, secret);
    let cells = e.card(60);
    let drawn: String = cells.iter().map(|c| c.c).collect();
    for ch in secret.chars().filter(|c| !c.is_whitespace() && *c != '-') {
        assert!(!drawn.contains(ch) || "ANTHROPICKEY_".contains(ch),
            "character {ch:?} of the secret reached the screen");
    }
    assert_eq!(
        cells.iter().filter(|c| c.c == '•').count(),
        secret.chars().count(),
        "one mask glyph per typed character"
    );
    assert!(drawn.contains("ANTHROPIC_API_KEY"), "the legend names the variable");
}

#[test]
fn a_long_key_never_overflows_the_card() {
    let mut e = KeyEntry::new("ANTHROPIC_API_KEY".into());
    typed(&mut e, &"x".repeat(500));
    let cols = 40u16;
    let cells = e.card(cols);
    assert!(cells.iter().all(|c| c.col < cols), "a cell escaped the card");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app keyentry`
Expected: compile error — the module does not exist.

- [ ] **Step 3: Write the module**

Create `crates/crew-app/src/keyentry.rs`:

```rust
//! The masked provider-key prompt. Opens when a model row is accepted that the
//! active stack can't serve for want of a key (`Route::needs_key`), so the
//! answer to "this needs ANTHROPIC_API_KEY" is a field rather than a trip to a
//! shell rc file and a restart.
//!
//! Modal by construction: while it is open every key belongs to it (see
//! [`KeyEntry::key`]), so nothing leaks to the pane underneath while a secret
//! is half-typed.
//!
//! The buffer is NEVER rendered in plaintext, logged, exported or written
//! anywhere but the credential store.
use crew_render::CellView;

use crate::chatkeys::ChatInput;

/// Total height: top border, one input row, bottom border.
pub(crate) const ROWS: u16 = 3;

/// What one key did to the prompt.
pub(crate) enum KeyOutcome {
    /// Handled; the prompt stays open. Every key that isn't Enter or Escape
    /// lands here, including ones the prompt ignores.
    Consumed,
    /// Escape: discard the buffer and close.
    Cancelled,
    /// Enter on a non-blank buffer: the trimmed key.
    Submit(String),
}

pub(crate) struct KeyEntry {
    /// The variable being supplied, e.g. `ANTHROPIC_API_KEY`. Shown; the
    /// buffer is not.
    pub(crate) var: String,
    buf: String,
}

impl KeyEntry {
    pub(crate) fn new(var: String) -> Self {
        Self {
            var,
            buf: String::new(),
        }
    }

    /// Route one key. Enter submits a non-blank buffer, Escape cancels,
    /// Backspace deletes, printable characters append (a paste arrives as a
    /// run of `Char`s). EVERYTHING else is swallowed rather than forwarded —
    /// this prompt is modal.
    pub(crate) fn key(&mut self, k: &ChatInput) -> KeyOutcome {
        match k {
            ChatInput::Char(c) => {
                self.buf.push(*c);
                KeyOutcome::Consumed
            }
            ChatInput::Backspace => {
                self.buf.pop();
                KeyOutcome::Consumed
            }
            ChatInput::Close => KeyOutcome::Cancelled,
            ChatInput::Enter => {
                // Pasted keys commonly carry a trailing newline or space.
                let v = self.buf.trim().to_string();
                if v.is_empty() {
                    KeyOutcome::Consumed
                } else {
                    KeyOutcome::Submit(v)
                }
            }
            _ => KeyOutcome::Consumed,
        }
    }

    /// The prompt as a fieldset card — a bordered box with the variable named
    /// in the legend, matching every other panel on the canvas rather than
    /// floating above it. The interior is one row of mask glyphs, one per
    /// typed character, clipped to the card's width.
    pub(crate) fn card(&self, cols: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        let title = format!("paste {}", self.var);
        let mut cells = crate::boxdraw::titled_card(
            cols,
            ROWS,
            &title,
            t.border_normal,
            t.legend_off,
            t.page_bg,
        );
        if cells.is_empty() {
            return cells;
        }
        let inner = cols.saturating_sub(2) as usize;
        for i in 0..self.buf.chars().count().min(inner) {
            cells.push(CellView {
                col: 1 + i as u16,
                row: 1,
                c: '•',
                fg: t.ink,
                bg: t.page_bg,
                bold: false,
                italic: false,
            });
        }
        cells
    }
}

#[cfg(test)]
#[path = "keyentry_tests.rs"]
mod tests;
```

Register it in `crates/crew-app/src/main.rs`. The `mod` list is alphabetical; `keyentry` sorts after `inputkeys` and before the next entry — insert it in its alphabetical slot.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p crew-app keyentry`
Expected: PASS, all seven tests.

- [ ] **Step 5: Lint and commit**

Run: `cargo clippy --workspace --all-targets -- -D warnings` — expect clean. `KeyEntry` has no caller until Task 5; if `dead_code` fires, do NOT add `#[allow]` and do NOT delete anything — fold Task 5 into this commit and say so in the commit message.

```bash
git add crates/crew-app/src/keyentry.rs crates/crew-app/src/keyentry_tests.rs \
        crates/crew-app/src/main.rs
git commit -m "feat(chat): masked provider-key prompt"
```

---

### Task 5: Wire the prompt into the pane

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (`ChatPane.keyentry`, routing, the `NeedsKey` arm, submit handling)
- Modify: `crates/crew-app/src/render.rs` (draw the card)
- Modify: `crates/crew-app/src/chat_tests.rs` (new tests)

**Interfaces:**
- Consumes: `keyentry::{KeyEntry, KeyOutcome, ROWS}` (Task 4), `chatpalette::PaletteKey::NeedsKey` (Task 3), `crew_plugin::credentials::{save_key, provider_for}` (Task 1), `crate::shellprobe::note_key` (added here).
- Produces: nothing — this is the last task.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/chat_tests.rs`:

```rust
// `pane()` is the existing helper at the top of this file — it spawns an idle
// child as a stand-in broker and returns a `ChatPane`. Use it; do NOT call
// `ChatPane::new` directly (it takes a `Plugin` and a channel).

#[test]
fn the_key_prompt_is_modal_and_escape_closes_it_not_the_pane() {
    let mut p = pane();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
    assert!(p.on_input(ChatInput::Close, std::path::Path::new(".")).is_none());
    assert!(p.keyentry.is_none(), "escape closes the prompt");
}

#[test]
fn the_prompt_swallows_keys_meant_for_the_composer() {
    let mut p = pane();
    let before = p.input.clone();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
    p.on_input(ChatInput::Char('x'), std::path::Path::new("."));
    assert_eq!(p.input, before, "a typed key must not reach the composer");
    assert!(p.keyentry.is_some(), "the prompt stays open");
}
```

Add to `crates/crew-app/src/shellprobe_tests.rs` (or create it and attach it):

```rust
#[test]
fn an_entered_key_joins_the_probed_set() {
    let mut keys: std::collections::HashSet<String> =
        ["OPENROUTER_API_KEY".to_string()].into_iter().collect();
    merge_entered(&mut keys, &[("ANTHROPIC_API_KEY".to_string(), "sk".to_string())]);
    assert!(keys.contains("ANTHROPIC_API_KEY"));
    assert!(keys.contains("OPENROUTER_API_KEY"), "probed keys survive");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p crew-app chat_tests && cargo test -p crew-app shellprobe`
Expected: compile errors — no `ChatPane.keyentry`, no `merge_entered`.

- [ ] **Step 3: Add the probe overlay**

In `crates/crew-app/src/shellprobe.rs`:

```rust
/// Keys supplied from inside crew this session, plus whatever the credential
/// store already held. The shell probe's cache is a `OnceLock` and can't be
/// re-set, so rather than convert it, entered keys are unioned over it.
static ENTERED: std::sync::RwLock<Vec<(String, String)>> = std::sync::RwLock::new(Vec::new());

/// Record a key supplied in-app so [`provider_now`] resolves against it
/// immediately. NEVER logs the value.
pub(crate) fn note_key(var: &str, value: &str) {
    if let Ok(mut e) = ENTERED.write() {
        e.retain(|(k, _)| k != var);
        e.push((var.to_string(), value.to_string()));
    }
}

/// Union entered keys into a probed key set (the testable half).
fn merge_entered(keys: &mut HashSet<String>, entered: &[(String, String)]) {
    for (k, v) in entered {
        if !v.is_empty() {
            keys.insert(k.clone());
        }
    }
}
```

Seed it from the store on first use and union it in `provider_now`:

```rust
pub(crate) fn provider_now() -> (Option<crew_plugin::Provider>, bool) {
    let Some(probed) = SHELL_PROBE.get() else {
        return (None, false);
    };
    let mut keys = probed.keys.clone();
    if let Ok(entered) = ENTERED.read() {
        merge_entered(&mut keys, &entered);
    }
    (resolve(&keys, probed.provider_pin.as_deref()), true)
}
```

Seed `ENTERED` from `crew_plugin::credentials::load()` once at app start — call `note_key` for each stored key from the same place the shell probe is kicked off, so keys entered in an earlier session make their rows live immediately.

**Known limitation, do not "fix" it here:** the `probed` flag is unchanged, so before the shell probe lands every row still renders `Unknown` even with a key in the overlay. crew does not claim a route on evidence it hasn't finished gathering, and the stored key reaches the broker regardless because the broker reads the file itself.

- [ ] **Step 4: Wire the pane**

In `crates/crew-app/src/chat.rs`, add the field to `ChatPane` (beside `palette`) and initialise it to `None` in the constructor:

```rust
    /// The masked provider-key prompt while one is open (see `keyentry`).
    /// Modal: it takes every key before the palette, the mention popup and the
    /// pane's own handling.
    pub(crate) keyentry: Option<crate::keyentry::KeyEntry>,
```

At the very top of `on_input` — **before** the palette block, since this prompt is modal and holds a half-typed secret:

```rust
        if let Some(entry) = self.keyentry.as_mut() {
            match entry.key(&k) {
                crate::keyentry::KeyOutcome::Consumed => return None,
                crate::keyentry::KeyOutcome::Cancelled => {
                    self.keyentry = None;
                    return None;
                }
                crate::keyentry::KeyOutcome::Submit(value) => {
                    let var = entry.var.clone();
                    self.keyentry = None;
                    self.store_provider_key(&var, &value);
                    return None;
                }
            }
        }
```

Replace Task 3's placeholder arm:

```rust
            crate::chatpalette::PaletteKey::NeedsKey(var) => {
                self.keyentry = Some(crate::keyentry::KeyEntry::new(var));
                return None;
            }
```

And add the submit handler. It never puts the value in a message:

```rust
    /// Persist a provider key supplied in-app, pin its provider, and make the
    /// picker resolve against it immediately. Reports what happened by NAME —
    /// the value never reaches a message, a log or the transcript.
    fn store_provider_key(&mut self, var: &str, value: &str) {
        let provider = crew_plugin::credentials::provider_for(var);
        let line = match crew_plugin::credentials::save_key(var, value, provider) {
            Ok(()) => {
                crate::shellprobe::note_key(var, value);
                match provider {
                    Some(p) => format!("{var} saved · {p} pinned"),
                    None => format!("{var} saved"),
                }
            }
            Err(e) => format!("could not save {var}: {e}"),
        };
        self.push_note(line);
    }
```

`push_note` (`chat.rs:127`) is the pane's existing helper for a local notice — it is what `/theme`, `/export` and the `/font` echo already use. Do not invent a new mechanism.

- [ ] **Step 5: Draw the card**

In `crates/crew-app/src/render.rs`, add a block mirroring the palette block at `render.rs:196-232` — same pane lookup, same positioning above the composer, `overlay: true`, using `crate::keyentry::ROWS` for the height and `entry.card(cols)` for the cells. It must be **mutually exclusive** with the palette block: when `keyentry` is open the palette is not drawn, matching the modal key routing.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: PASS.

- [ ] **Step 7: Lint and check the file budget**

Run: `cargo clippy --workspace --all-targets -- -D warnings` — expect clean.
Run: `wc -l crates/crew-app/src/chat.rs crates/crew-app/src/keyentry.rs crates/crew-app/src/render.rs`
If `chat.rs` has gone well past its previous size, move `store_provider_key` to a sibling module rather than leaving it over budget.

- [ ] **Step 8: Verify no secret can reach disk or screen**

Run: `grep -rn "value\|buf" crates/crew-app/src/keyentry.rs crates/crew-app/src/chat.rs | grep -iE "println|eprintln|tracing|log::|format!\(\"\{value"`
Expected: no hit that would render or log a key value. This is a manual check, not a test — record what you ran and what it returned.

- [ ] **Step 9: Commit**

```bash
git add crates/crew-app/src
git commit -m "feat(chat): store a provider key from the pane and route to it immediately"
```

---

## Verification (after all five tasks)

- [ ] `cargo test --workspace` — green.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — green.
- [ ] Launch `target/debug/crew` (a dev launch spawns ITSELF as the broker via `current_exe()`; NEVER overwrite `~/.local/bin/crew`, the user's working install). In a `/smith` pane type `/model`, pick a dimmed row, confirm the prompt names the right variable, type a fake key, and confirm the pane reports `<VAR> saved · <provider> pinned` and that the typed characters appeared only as `•`.
- [ ] Confirm `<config_dir>/crew/credentials.json` exists with mode `0600` and contains the fake key, then **delete it** so the fake key is not left behind.
- [ ] Confirm Escape at the prompt closes the prompt and not the pane.
