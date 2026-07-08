# Smart Input Bar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bare text in the input bar routes itself: idle focused shell → typed there; otherwise PATH-verified commands spawn a new pane; `*` broadcasts one-shot; typos hint instead of spawning. Spec: `docs/superpowers/specs/2026-07-07-smart-input-bar-design.md`.

**Architecture:** A pure command-resolution module (`cmdcheck`), a pure routing decision (`route`), and a rewritten tail of `CrewApp::submit_input`. The idle probe already exists (`PtyTerm::foreground_pid()`). The palette preview row is display-only, built in `render.rs` where app context lives — `inputkeys.rs` menu handling is untouched, so Enter semantics live only in `submit_input`.

**Tech Stack:** Rust, winit app (`crates/crew-app`), existing `portable-pty` plumbing in `crates/crew-term`.

## Global Constraints

- Never block the winit main thread (login-shell PATH capture runs on a spawned thread; per-keystroke work is a handful of `stat`s).
- Pre-commit runs `cargo fmt --check` and `cargo check`; run `cargo fmt` before each commit.
- Commit messages: repo style `feat(crew-app): …` / `fix(crew-app): …`, ending with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- All tests: `cargo test -p crew-app` must stay green after every task.
- Comments follow the repo voice: explain constraints/invariants, not what the next line does.

---

### Task 1: `cmdcheck` — command resolution

**Files:**
- Create: `crates/crew-app/src/cmdcheck.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod cmdcheck;` in the alphabetical mod list, after `mod cmdmenu;`)

**Interfaces:**
- Produces: `pub(crate) enum Verdict { Executable(String), Builtin(String), No }` (derives `Clone, Debug, PartialEq, Eq`), `pub(crate) fn first_word(line: &str) -> Option<String>`, `pub(crate) fn resolve(line: &str, path: &str) -> Verdict`. `path` is an injected `:`-separated dir list so tests never depend on the host PATH.

- [ ] **Step 1: Write the failing tests** (bottom of the new `cmdcheck.rs`; module body from Step 3 not yet present — create the file with only a stub `pub(crate) enum Verdict …` + tests if you prefer, or write tests first and let the compile fail)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// A temp dir holding one executable `hit` and one plain file `miss`.
    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let hit = d.path().join("hit");
        std::fs::write(&hit, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hit, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(d.path().join("miss"), "").unwrap();
        d
    }

    #[test]
    fn first_word_strips_env_prefixes_and_quotes() {
        assert_eq!(first_word("FOO=1 BAR=2 cargo test"), Some("cargo".into()));
        assert_eq!(first_word("\"hit\" --flag"), Some("hit".into()));
        assert_eq!(first_word("  ls -la"), Some("ls".into()));
        assert_eq!(first_word("FOO=1"), None, "only assignments → no command word");
        assert_eq!(first_word(""), None);
    }

    #[test]
    fn resolve_finds_executables_on_the_given_path() {
        let d = fixture();
        let path = d.path().to_str().unwrap().to_string();
        assert_eq!(resolve("hit --flag", &path), Verdict::Executable("hit".into()));
        assert_eq!(resolve("miss", &path), Verdict::No, "non-executable file");
        assert_eq!(resolve("nosuch", &path), Verdict::No);
    }

    #[test]
    fn resolve_accepts_explicit_paths_and_rejects_bad_ones() {
        let d = fixture();
        let hit = d.path().join("hit");
        assert_eq!(
            resolve(hit.to_str().unwrap(), ""),
            Verdict::Executable("hit".into()),
            "absolute path bypasses PATH"
        );
        assert_eq!(resolve("./nosuch/prog", ""), Verdict::No);
    }

    #[test]
    fn resolve_flags_shell_builtins() {
        assert_eq!(resolve("export FOO=1", ""), Verdict::Builtin("export".into()));
        assert_eq!(resolve("source ~/.zshrc", ""), Verdict::Builtin("source".into()));
    }

    #[test]
    fn effective_path_falls_back_to_process_path() {
        // Hydration hasn't run in tests; must equal the process PATH, not panic.
        assert_eq!(effective_path(), std::env::var("PATH").unwrap_or_default());
    }
}
```

- [ ] **Step 2: Add `tempfile` dev-dependency and run tests to verify they fail**

Check whether `tempfile` is already a dev-dependency: `rg -n "tempfile" crates/crew-app/Cargo.toml Cargo.toml`. If absent, add under `[dev-dependencies]` in `crates/crew-app/Cargo.toml`:

```toml
tempfile = "3"
```

Run: `cargo test -p crew-app cmdcheck 2>&1 | tail -5`
Expected: compile FAILURE (`first_word` / `resolve` not defined).

- [ ] **Step 3: Write the implementation** (top of `cmdcheck.rs`, above the tests)

```rust
//! Is this line a runnable command? Powers the input bar's smart routing:
//! the first word must resolve to a real executable (hydrated login-shell
//! PATH, explicit path) or a shell builtin before crew will spawn a pane
//! for it — so typos hint instead of littering dead panes.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// What the first word of an input line turned out to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Resolves to an executable (name is the bare first word).
    Executable(String),
    /// A shell builtin that would be pointless in a throwaway pane.
    Builtin(String),
    /// Not something we can run.
    No,
}

/// State-mutating builtins: running them in a fresh pane silently does
/// nothing useful (the pane's shell exits with the state). `cd` is handled
/// earlier in submit_input, `echo`/`printf` etc. exist as real binaries.
const BUILTINS: &[&str] = &[
    "export", "set", "unset", "source", ".", "alias", "unalias", "eval",
];

/// The command word of `line`: the first whitespace token after skipping
/// leading `VAR=value` assignments, with surrounding quotes stripped.
pub(crate) fn first_word(line: &str) -> Option<String> {
    let word = line
        .split_whitespace()
        .find(|t| !is_assignment(t))?
        .trim_matches(|c| c == '"' || c == '\'');
    (!word.is_empty()).then(|| word.to_string())
}

/// `FOO=bar` (an env prefix), as opposed to a command word.
fn is_assignment(token: &str) -> bool {
    match token.split_once('=') {
        Some((name, _)) => {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        None => false,
    }
}

/// Classify `line` against the `:`-separated `path` dir list.
pub(crate) fn resolve(line: &str, path: &str) -> Verdict {
    let Some(word) = first_word(line) else {
        return Verdict::No;
    };
    if BUILTINS.contains(&word.as_str()) {
        return Verdict::Builtin(word);
    }
    if word.contains('/') {
        let p = expand_home(&word);
        return if is_executable(&p) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or(word);
            Verdict::Executable(name)
        } else {
            Verdict::No
        };
    }
    for dir in path.split(':').filter(|d| !d.is_empty()) {
        if is_executable(&Path::new(dir).join(&word)) {
            return Verdict::Executable(word);
        }
    }
    Verdict::No
}

fn expand_home(word: &str) -> PathBuf {
    if let Some(rest) = word.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(word)
}

/// Executable regular file. On non-Unix there is no mode bit; existence of a
/// file is the best cheap signal.
fn is_executable(p: &Path) -> bool {
    let Ok(md) = std::fs::metadata(p) else {
        return false;
    };
    if !md.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        md.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Login-shell PATH captured once by [`init_shell_path`]. Dock-launched crew
/// inherits launchd's minimal PATH, so a command like `claude` in
/// `~/.local/bin` would *run* fine (spawns go through `$SHELL -c`) yet fail
/// detection without this.
static SHELL_PATH: OnceLock<String> = OnceLock::new();

/// Capture `$SHELL -lc 'printf %s "$PATH"'` on a background thread (the winit
/// thread must never block on a subprocess). `CREW_SHELL_ENV=0` skips it,
/// mirroring the broker's env hydration switch.
pub(crate) fn init_shell_path() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        return;
    }
    std::thread::spawn(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let Ok(out) = std::process::Command::new(&shell)
            .args(["-lc", "printf %s \"$PATH\""])
            .output()
        else {
            return;
        };
        if !out.status.success() {
            return;
        }
        if let Ok(p) = String::from_utf8(out.stdout) {
            if !p.trim().is_empty() {
                let _ = SHELL_PATH.set(p);
            }
        }
    });
}

/// The PATH detection resolves against: hydrated login-shell PATH once it
/// lands, the process PATH until then.
pub(crate) fn effective_path() -> String {
    SHELL_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default())
}
```

Register the module in `crates/crew-app/src/main.rs` (alphabetical, after `mod cmdmenu;`):

```rust
mod cmdcheck;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app cmdcheck 2>&1 | tail -5`
Expected: `test result: ok. 5 passed`

- [ ] **Step 5: Wire `init_shell_path` into startup and commit**

In `crates/crew-app/src/main.rs`, add as the first line of `fn main()`:

```rust
cmdcheck::init_shell_path();
```

Run: `cargo fmt && cargo test -p crew-app 2>&1 | tail -3` (all green), then:

```bash
git add crates/crew-app/src/cmdcheck.rs crates/crew-app/src/main.rs crates/crew-app/Cargo.toml Cargo.lock
git commit -m "feat(crew-app): cmdcheck — resolve input-bar text against a hydrated PATH

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: `*` broadcast prefix + explicit terminal write targets

**Files:**
- Modify: `crates/crew-app/src/app.rs` (add `star_command` next to `bang_command` at app.rs:244; add the `*` branch in `submit_input`)
- Modify: `crates/crew-app/src/termwrite.rs` (parameterize the write target)

**Interfaces:**
- Consumes: `submit_bytes` (app.rs:253), `set_status` (status.rs:20).
- Produces: `pub(crate) fn star_command(line: &str) -> Option<&str>` (app.rs); `pub(crate) fn write_terminal_targets(&mut self, bytes: &[u8], all: bool) -> usize` (termwrite.rs). Task 3 routes rule 6 through `write_terminal_targets(bytes, false)`.

- [ ] **Step 1: Write the failing tests** (append to the `#[cfg(test)] mod tests` at the bottom of `app.rs` — it exists at app.rs:264 area; the app-level test goes in `app_tests.rs` alongside its existing `CrewApp::default()` tests)

In `app.rs` tests:

```rust
#[test]
fn star_command_strips_the_prefix() {
    assert_eq!(star_command("* ls -la"), Some("ls -la"));
    assert_eq!(star_command("*ls"), Some("ls"));
    assert_eq!(star_command("*"), Some(""));
    assert_eq!(star_command("ls *"), None);
}
```

In `app_tests.rs`:

```rust
#[test]
fn star_broadcast_with_no_terminals_hints() {
    let mut app = CrewApp::default();
    app.submit_input("* echo hi".into());
    let status = app.status.as_ref().map(|(m, _)| m.clone()).unwrap_or_default();
    assert!(status.contains("no terminals"), "got: {status}");
}
```

(If `status` is not directly readable, follow the pattern the existing `app_tests.rs` uses for status assertions — `rg -n "set_status\|status" crates/crew-app/src/app_tests.rs` first, and mirror it.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app star 2>&1 | tail -5`
Expected: compile FAILURE (`star_command` not defined).

- [ ] **Step 3: Implement**

In `app.rs`, below `bang_command` (app.rs:244-246):

```rust
/// If `line` is a `*text` broadcast, return the trimmed payload (empty when
/// just `*`); else `None`. The payload is sent to EVERY terminal pane —
/// broadcast is an explicit prefix, not a mode, so nothing else the bar does
/// depends on Cmd+S state.
pub(crate) fn star_command(line: &str) -> Option<&str> {
    line.strip_prefix('*').map(str::trim)
}
```

In `termwrite.rs`, split the existing `write_to_terminals` (keep its doc comment style):

```rust
/// Write `bytes` to terminal panes: the focused one, or — when `all` — every
/// terminal pane. Each write snaps to the bottom. Returns how many terminals
/// received it (0 means nothing did, e.g. no shell is open/focused).
pub(crate) fn write_terminal_targets(&mut self, bytes: &[u8], all: bool) -> usize {
    let focused = self.focused;
    let mut count = 0;
    for (i, pane) in self.panes.iter_mut().enumerate() {
        if !all && i != focused {
            continue;
        }
        if let PaneContent::Terminal(t) = &mut pane.content {
            t.pty.scroll_to_bottom();
            // Typing invalidates any mouse selection — drop the stale
            // highlight so it doesn't linger painted over fresh output.
            t.pty.sel_clear();
            if let Err(e) = t.input.write_all(bytes).and_then(|_| t.input.flush()) {
                eprintln!("terminal write error: {e}");
            } else {
                count += 1;
            }
        }
    }
    count
}

/// Keystrokes typed while a terminal pane is focused: honors Cmd+S broadcast
/// (synchronized typing). The input bar does NOT come through here — its
/// routing never consults the mode.
pub(crate) fn write_to_terminals(&mut self, bytes: &[u8]) -> usize {
    let all = self.broadcast;
    self.write_terminal_targets(bytes, all)
}
```

In `submit_input` (app.rs:173), after the `bang_command` block (app.rs:183-190) and **before** `try_change_dir`:

```rust
// `*text` broadcasts one line to every terminal pane, explicitly — the
// bar's replacement for depending on Cmd+S broadcast mode.
if let Some(cmd) = star_command(&line) {
    if cmd.is_empty() {
        self.set_status("usage: *<text> — sends to every terminal");
    } else if self.write_terminal_targets(&submit_bytes(cmd), true) == 0 {
        self.set_status("no terminals to broadcast to");
    }
    return false;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app 2>&1 | tail -3`
Expected: all green including the two new tests.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/app.rs crates/crew-app/src/app_tests.rs crates/crew-app/src/termwrite.rs
git commit -m "feat(crew-app): * prefix broadcasts one line to every terminal

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: bare-text routing — idle shell, spawn, hints

**Files:**
- Create: `crates/crew-app/src/route.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod route;` alphabetically, after `mod render;`)
- Modify: `crates/crew-app/src/app.rs` (`submit_input` tail rewrite; add `cmd_cache` field on `CrewApp`, `check_command`, `focused_target` helpers)

**Interfaces:**
- Consumes: `cmdcheck::{Verdict, first_word, resolve, effective_path}` (Task 1), `write_terminal_targets` (Task 2), `run_in_pane` (runpane.rs:34), `PtyTerm::foreground_pid()` (crew-term pty.rs:380, already public).
- Produces: `pub(crate) enum Target { IdleShell(usize), Other }`, `pub(crate) enum BareRoute { TypeInto(usize), Spawn, BuiltinHint(String), UnknownHint }`, `pub(crate) fn route_bare(target: Target, verdict: &Verdict) -> BareRoute` (route.rs); `CrewApp::focused_target() -> Target`, `CrewApp::check_command(&mut self, line: &str) -> Verdict` (app.rs). Task 4's preview consumes all three.

- [ ] **Step 1: Write the failing tests** (in `route.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmdcheck::Verdict;

    #[test]
    fn idle_shell_wins_over_everything() {
        // Even a resolvable command goes INTO an idle focused shell.
        let r = route_bare(Target::IdleShell(2), &Verdict::Executable("ls".into()));
        assert!(matches!(r, BareRoute::TypeInto(2)));
        // …and so does prose: the shell is the judge of what it means.
        let r = route_bare(Target::IdleShell(0), &Verdict::No);
        assert!(matches!(r, BareRoute::TypeInto(0)));
    }

    #[test]
    fn busy_or_nonterminal_focus_diverts_by_verdict() {
        assert!(matches!(
            route_bare(Target::Other, &Verdict::Executable("claude".into())),
            BareRoute::Spawn
        ));
        assert!(matches!(
            route_bare(Target::Other, &Verdict::Builtin("export".into())),
            BareRoute::BuiltinHint(b) if b == "export"
        ));
        assert!(matches!(
            route_bare(Target::Other, &Verdict::No),
            BareRoute::UnknownHint
        ));
    }
}
```

And in `app_tests.rs` (Far pane focused = `Target::Other`; no real PTY needed):

```rust
#[test]
fn bare_nonsense_with_no_shell_hints_instead_of_spawning() {
    let mut app = CrewApp::default();
    app.panes.push(far_pane("files")); // reuse the existing far_pane helper
    app.focused = 0;
    app.submit_input("definitely-not-a-command-xyz".into());
    assert_eq!(app.panes.len(), 1, "no junk pane spawned");
    let status = app.status.as_ref().map(|(m, _)| m.clone()).unwrap_or_default();
    assert!(status.contains("not a command"), "got: {status}");
}
```

(Check first how `app_tests.rs` builds panes — `rg -n "far_pane\|fn.*pane\(" crates/crew-app/src/app_tests.rs` — and reuse its helper; `panemanage.rs` tests have one to copy if `app_tests.rs` lacks it.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app route 2>&1 | tail -5`
Expected: compile FAILURE (`route` module missing).

- [ ] **Step 3: Implement `route.rs`**

```rust
//! Where a bare (un-prefixed) input-bar line goes. Pure decision so every
//! row of the spec's routing table is unit-testable; `submit_input` supplies
//! the two inputs and acts on the answer.
use crate::cmdcheck::Verdict;

/// The focused pane, as routing sees it: a terminal whose shell owns the
/// prompt (idle), or anything else — busy terminal, chat/md/settings pane,
/// hidden pane, or no pane at all.
pub(crate) enum Target {
    IdleShell(usize),
    Other,
}

/// The routing decision for a bare line.
pub(crate) enum BareRoute {
    /// Type the line into the idle focused shell (pane index).
    TypeInto(usize),
    /// Spawn a new persistent pane running the line.
    Spawn,
    /// Shell builtin — a throwaway pane would discard its effect; hint.
    BuiltinHint(String),
    /// Unresolvable — hint instead of spawning a dead pane.
    UnknownHint,
}

/// Focused-shell-first: an idle shell receives anything (it is the judge of
/// what the text means); everything else routes by what the first word is.
pub(crate) fn route_bare(target: Target, verdict: &Verdict) -> BareRoute {
    if let Target::IdleShell(i) = target {
        return BareRoute::TypeInto(i);
    }
    match verdict {
        Verdict::Executable(_) => BareRoute::Spawn,
        Verdict::Builtin(b) => BareRoute::BuiltinHint(b.clone()),
        Verdict::No => BareRoute::UnknownHint,
    }
}
```

- [ ] **Step 4: Rewrite the `submit_input` tail and add the helpers** (app.rs)

Replace the current tail of `submit_input` (app.rs:191-201 — the `try_change_dir` call stays, everything after it changes):

```rust
// `cd` in the input bar moves Crew's working directory, not the terminal's.
if self.try_change_dir(&line) {
    return false;
}
match crate::route::route_bare(self.focused_target(), &self.check_command(&line)) {
    crate::route::BareRoute::TypeInto(_) => {
        // The focused idle shell receives the line as keystrokes.
        if self.write_terminal_targets(&submit_bytes(&line), false) == 0 {
            self.set_status("no shell here — press Cmd+T to open one");
        }
    }
    crate::route::BareRoute::Spawn => self.run_in_pane(&line),
    crate::route::BareRoute::BuiltinHint(b) => {
        self.set_status(format!("{b} is a shell builtin — run it inside a shell pane"));
    }
    crate::route::BareRoute::UnknownHint => {
        self.set_status(format!("not a command — !{line} runs it in a pane anyway"));
    }
}
false
```

Add the helpers in the same `impl CrewApp` block (near `submit_input`):

```rust
/// The focused pane as routing sees it: `IdleShell` only for a visible
/// terminal whose shell owns the prompt (`foreground_pid()` is `None`).
/// Hidden panes are not "in the main area", so they never receive text.
pub(crate) fn focused_target(&self) -> crate::route::Target {
    if let Some(p) = self.panes.get(self.focused) {
        if !p.hidden {
            if let crate::pane::PaneContent::Terminal(t) = &p.content {
                if t.pty.foreground_pid().is_none() {
                    return crate::route::Target::IdleShell(self.focused);
                }
            }
        }
    }
    crate::route::Target::Other
}

/// Resolve `line`'s first word, memoized — the palette preview re-checks on
/// every keystroke and only the first word matters, so argument typing must
/// not re-stat the PATH.
pub(crate) fn check_command(&mut self, line: &str) -> crate::cmdcheck::Verdict {
    let word = crate::cmdcheck::first_word(line);
    if let (Some(w), Some((cached_w, v))) = (&word, &self.cmd_cache) {
        if w == cached_w {
            return v.clone();
        }
    }
    let v = crate::cmdcheck::resolve(line, &crate::cmdcheck::effective_path());
    if let Some(w) = word {
        self.cmd_cache = Some((w, v.clone()));
    }
    v
}
```

Add the field to the `CrewApp` struct (find it: `rg -n "pub(crate) struct CrewApp" -A 5 crates/crew-app/src/app.rs`; place near other transient fields like `scroll_accum`, and add `cmd_cache: None` to the `Default`/constructor):

```rust
/// Last resolved (first_word, verdict) — see [`Self::check_command`].
pub(crate) cmd_cache: Option<(String, crate::cmdcheck::Verdict)>,
```

Register `mod route;` in `main.rs`.

- [ ] **Step 5: Run all tests**

Run: `cargo test -p crew-app 2>&1 | tail -3`
Expected: all green. NOTE the pre-existing test at app.rs/app_tests.rs asserting "no shell here" behavior for prose (`submit_input` used to write prose to terminals): if a test now fails because prose no longer reaches a terminal, that test encodes the OLD spec — update it to assert the new `not a command` hint, and say so in the commit message.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/route.rs crates/crew-app/src/app.rs crates/crew-app/src/app_tests.rs crates/crew-app/src/main.rs
git commit -m "feat(crew-app): bare input routes smart — idle shell first, PATH-verified spawn, hints

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: palette preview row

**Files:**
- Modify: `crates/crew-app/src/route.rs` (add `input_preview` on `CrewApp` + pure label helpers)
- Modify: `crates/crew-app/src/render.rs` (build_frame's command-menu block, render.rs:177-192)

**Interfaces:**
- Consumes: `route_bare`, `focused_target`, `check_command` (Task 3), `star_command`/`bang_command` (app.rs), `cd_arg` (cwd.rs — check its visibility first: `rg -n "fn cd_arg" crates/crew-app/src/cwd.rs`; make it `pub(crate)` if it isn't), `suggest::MenuItem` (suggest.rs:16).
- Produces: `CrewApp::input_preview() -> Vec<crate::suggest::MenuItem>`; display-only — `inputkeys.rs` is deliberately untouched, so Enter/Tab/history behavior comes only from `submit_input`.

- [ ] **Step 1: Write the failing tests** (append to `route.rs` tests; drive `input_preview` through a real `CrewApp` with Far panes, mirroring Task 3's app test setup)

```rust
#[test]
fn preview_labels_spawn_and_hint_rows() {
    let mut app = crate::app::CrewApp::default();
    app.panes.push(crate::route::tests::far_pane("files"));
    app.focused = 0;
    // Resolvable → a submit row naming the new pane destination.
    app.input.text = "/bin/echo hi".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("new pane"), "got: {}", rows[0].label);
    assert!(rows[0].submit);
    // Unresolvable → a dim non-submit hint row.
    app.input.text = "definitely-not-a-command-xyz".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].submit);
    assert!(rows[0].label.contains("not a command"), "got: {}", rows[0].label);
}

#[test]
fn preview_is_silent_for_slash_cd_and_empty() {
    let mut app = crate::app::CrewApp::default();
    app.input.text = "/theme".into();
    assert!(app.input_preview().is_empty(), "slash palette owns / input");
    app.input.text = "cd ~/code".into();
    assert!(app.input_preview().is_empty(), "cd keeps its ghost, no card");
    app.input.text = String::new();
    assert!(app.input_preview().is_empty());
}

#[test]
fn preview_counts_broadcast_targets() {
    let mut app = crate::app::CrewApp::default();
    app.input.text = "* echo hi".into();
    let rows = app.input_preview();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("0 terminals"), "got: {}", rows[0].label);
}
```

(Adjust the `far_pane` reference to wherever the shared helper ends up — if `app_tests.rs` has it privately, lift it into a `#[cfg(test)] pub(crate) fn far_pane` in one place both test modules import.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app preview 2>&1 | tail -5`
Expected: compile FAILURE (`input_preview` not defined).

- [ ] **Step 3: Implement `input_preview`** (in `route.rs`, an `impl CrewApp` block)

```rust
impl crate::app::CrewApp {
    /// The palette's live answer to "what will Enter do with this text?" —
    /// zero rows for input another surface owns (slash palette, cd ghost,
    /// empty), one row otherwise. Display-only: Enter semantics live solely
    /// in `submit_input`, this row just mirrors them (`fill` = the text, so
    /// even a stray menu-Enter is identical to a plain submit).
    pub(crate) fn input_preview(&mut self) -> Vec<crate::suggest::MenuItem> {
        use crate::suggest::MenuItem;
        let text = self.input.text.clone();
        if text.is_empty() || text.starts_with('/') {
            return Vec::new();
        }
        let row = |label: String, desc: &str, submit: bool| {
            vec![MenuItem {
                label,
                desc: desc.to_string(),
                fill: text.clone(),
                submit,
            }]
        };
        if crate::app::star_command(&text).is_some() {
            let n = self
                .panes
                .iter()
                .filter(|p| matches!(p.content, crate::pane::PaneContent::Terminal(_)))
                .count();
            return row(format!("↵ broadcast to {n} terminals"), "", true);
        }
        if crate::app::bang_command(&text).is_some() {
            return row("↵ run in a new pane (forced)".to_string(), "", true);
        }
        if crate::cwd::cd_arg(&text).is_some() {
            return Vec::new();
        }
        match route_bare(self.focused_target(), &self.check_command(&text)) {
            BareRoute::TypeInto(i) => {
                let title = self
                    .panes
                    .get(i)
                    .map(|p| p.title_text())
                    .unwrap_or_default();
                row(format!("↵ type into pane {} · {title}", i + 1), "", true)
            }
            BareRoute::Spawn => row("↵ run — new pane".to_string(), "", true),
            BareRoute::BuiltinHint(b) => row(
                format!("{b} is a shell builtin — run it in a shell pane"),
                "",
                false,
            ),
            BareRoute::UnknownHint => row(
                "not a command — !… runs it in a pane anyway".to_string(),
                "",
                false,
            ),
        }
    }
}
```

- [ ] **Step 4: Wire into `render.rs`** (the command-menu block at render.rs:177)

Replace:

```rust
let matches = crate::suggest::menu_items(&self.input.text);
```

with:

```rust
let matches = if self.input.text.starts_with('/') {
    crate::suggest::menu_items(&self.input.text)
} else {
    self.input_preview()
};
```

The card title two lines below (render.rs:183, `menu_card("commands", …)`) becomes:

```rust
let title = if self.input.text.starts_with('/') { "commands" } else { "input" };
```

and pass `title` to `menu_card`.

- [ ] **Step 5: Run all tests, then commit**

Run: `cargo fmt && cargo test -p crew-app 2>&1 | tail -3`
Expected: all green.

```bash
git add crates/crew-app/src/route.rs crates/crew-app/src/render.rs crates/crew-app/src/cwd.rs
git commit -m "feat(crew-app): palette preview row shows where bare input will go

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: palette cleanup + help

**Files:**
- Modify: `crates/crew-app/src/suggest.rs` (COMMANDS list, suggest.rs:93)
- Modify: `crates/crew-app/src/help.rs` (BINDINGS, help.rs:13)

**Interfaces:**
- Consumes: nothing new. Dispatch (`run_slash_command`) keeps `/shell` and `/run` — only their palette rows go.

- [ ] **Step 1: Write the failing test** (in `suggest.rs`'s existing test module)

```rust
#[test]
fn palette_hides_shell_and_run_but_dispatch_keeps_them() {
    assert!(!COMMANDS.iter().any(|c| c.name == "/shell" || c.name == "/run"),
        "bare text replaced these palette rows");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app palette_hides 2>&1 | tail -5`
Expected: FAIL (entries still present).

- [ ] **Step 3: Implement**

- Delete the `/shell` and `/run` `Cmd { … }` entries from `COMMANDS` (suggest.rs:99-101 and 106-108).
- Update the comment above `COMMANDS`: `/// Known slash commands (kept in sync with run_slash_command; /shell and /run stay dispatchable but bare text replaced their palette rows).`
- In `help.rs` BINDINGS, after the `("/ (in input)", "Command palette")` row add:

```rust
("! · * (in input)", "Force a new pane / broadcast to all terminals"),
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p crew-app 2>&1 | tail -3`
Expected: all green (help overlay sizes itself from the lists; no other test should care).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-app/src/suggest.rs crates/crew-app/src/help.rs
git commit -m "feat(crew-app): drop /shell and /run from the palette; document ! and * prefixes

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: live verification

**Files:** none (verification only). Follow `.claude/skills/verify/SKILL.md` **exactly** — especially the frontmost-PID guard: the Claude session may be running inside the user's live crew app, and unguarded keystrokes land in their agent chats.

- [ ] **Step 1: Build and launch isolated**

```bash
cargo build -p crew-app
# isolated HOME, then warm relaunch per the skill (first cold launch ~2 min, unfocused)
```

- [ ] **Step 2: Drive the routing table** (each keystroke batch behind the PID guard)

1. Fresh instance, no panes: type `ls` + Enter → a new pane spawns, runs `ls`, drops to a prompt (rule 7 with empty grid).
2. With that shell pane focused and idle: type `git status` + Enter → runs **in that pane**, no new pane (rule 6).
3. Type `top` in the idle shell → runs there. Quit top (`q`). Now `/md README.md` (zoomed md viewer focused), type `top` + Enter → **new pane** running top (rule 7 diverts). Screenshot.
4. Type `caude --help` (typo) with the md pane focused → status hint "not a command", no pane spawned (rule 9). Screenshot the palette hint row mid-typing.
5. `* echo hello` with two shell panes open → both prompts show `hello` (rule 5). Screenshot.
6. `export FOO=1` with md pane focused → builtin hint (rule 8).

- [ ] **Step 3: Screenshot the preview row states** (`↵ run … — new pane`, `↵ type into pane N`, broadcast count, dim hint) and kill the dev instance.

- [ ] **Step 4: Report per the verify skill format** (PASS/FAIL with evidence).

---

## Self-review notes

- Spec coverage: rules 1-9 → Tasks 2 (rules 3/5), 3 (rules 6-9; 1/2/4 untouched by design); detection + hydration → Task 1; preview → Task 4; `/shell`+`/run` palette exit and prefix docs → Task 5; live verify → Task 6. Broadcast-mode decoupling of the bar is the `write_to_terminals` → `write_terminal_targets(all=…)` split in Task 2.
- Type consistency: `Verdict` (Task 1) consumed by `route_bare` (Task 3) and `check_command` cache; `MenuItem` fields match suggest.rs:16-27.
- Known judgment call: `check_command` memoizes only the last word (one-entry cache) — enough to keep argument-typing from re-statting.
