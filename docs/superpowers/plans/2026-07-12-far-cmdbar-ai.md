# Far Command Bar: AI Suggestions — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Type `! <what you want>` in the Far pane's command bar and get a shell command back as an editable, highlighted suggestion — one provider call, zero cost/latency unless invoked, never auto-run. Executes after
`2026-07-12-far-cmdbar-complete.md` (Phase 1: Tab completion, history,
ghost text) has landed — this plan's diffs are written against Phase 1's
final `FarPane` shape (`history`, `complete`, `bins`, `bins_scan_started`
fields; `keys.rs`'s `typing`-gated `reduce()`; `run_cmdline`'s history push;
`render.rs`'s `ghost`-carrying `command_bar`).

**Architecture:** One new pure module, `farpane/ask.rs` (the `AskState`
enum + the `! ` trigger parser `bang_ask`), plus a new one-shot provider
call in `crew-plugin` (`suggest_far_command`, alongside the existing
`suggest_command`/`explain_output`). `FarPane` gains one field (`ask:
Option<ask::AskState>`) and two methods (`poll_ask`, `absorb_ask_result`)
that `keys.rs` wires into Enter/Esc/typing, `run.rs` feeds via a new
`submit_ask` off-thread runner (same `mpsc::Receiver` shape as `running`),
and `render.rs` paints inline in the command bar — a live `thinking… Ns`
while waiting, or the landed suggestion highlighted like the panel's
selected-row style with an `Enter run · Esc discard · keep typing to edit`
hint. `crew-app/src/poll.rs`'s existing Far-pane arm grows one more
`if let Some(msg) = f.poll_ask()`, mirroring its `poll_cmd()` line.

**Tech Stack:** Rust, `crew_hive::{Provider, CompletionRequest, ModelTier}`
(existing dep of `crew-plugin`), `tokio` current-thread runtime for the
one-shot blocking call (existing dep, same pattern as
`ApiAdapter::call_with_usage`), `std::sync::mpsc` + `std::thread` (existing
pattern from `farpane/run.rs`). **No new crates or `Cargo.toml` changes
anywhere** — `crew-app` already depends on `crew-plugin`, `crew-hive`, and
`tokio`; `crew-plugin` already depends on `crew-hive` and `tokio`.

## Provider-plumbing decision (resolved ambiguity — read before Task 1)

The spec proposes exposing a minimal `pub fn` over
`crew_plugin::broker::discover` (currently crate-internal). Investigation
found two existing candidates and neither fits as-is:

- **`crew_plugin::suggest_command`/`explain_output`** (`crew-plugin/src/broker/ask.rs`)
  already power the main input bar's `?`/`??` asks (`crew-app/src/askbar.rs`,
  in-process, no broker child — a worker thread calls the function directly
  and posts the result over an `mpsc::Receiver` that `CrewApp::poll_ask`
  drains every tick). This is the right *shape* to mirror. But
  `suggest_command` routes through `discover::roster_with` →
  `Adapter::call`, which pins every call to the **"coder" role's fixed
  system prompt** ("You are the coder. Implement…") and a **hardcoded
  2048-token ceiling** (`apiadapter::MAX_TOKENS`) — neither configurable
  per call. The spec needs an exact, different system prompt
  (cwd/OS-aware) and `max_tokens: 128`. Reusing `suggest_command` verbatim
  would silently drop both requirements.
- **`discover::roster_with`** itself returns `Vec<Box<dyn Adapter>>` — the
  `Adapter` trait has no accessor for the underlying `Arc<dyn Provider>` or
  model id, so there is no way to get a raw provider out of it without
  either widening `Adapter`'s public surface or bypassing it.

**Decision:** add a new `pub(crate) fn provider_and_model()` to
`discover.rs` that mirrors `roster_with`'s provider-selection branches
(mock / DashScope / OpenRouter / Anthropic) but stops **before**
`inbuilt_agents` — returning the raw `(Arc<dyn Provider>, model: String)`
instead of role-wrapped `Adapter`s. A new `pub fn suggest_far_command` in
`ask.rs` (same file/pattern as `suggest_command`) calls it directly with a
hand-built `CompletionRequest { system: Some(<spec's exact prompt>),
max_tokens: 128, .. }`, reusing `extract_command` (already `pub(crate)` in
the same file) for the fence/whitespace-stripping post-processing — so the
spec's "strips fences/whitespace to one line" requirement is met by
**reusing existing, already-tested code**, not reimplementing it.
`roster_with` itself is untouched — zero risk to the `/crew` broker's
existing behaviour or tests. `CREW_BROKER_MOCK_REPLY` short-circuits
automatically because `provider_and_model()` calls the same
`pick_provider()` the mock path already goes through.

This is a deliberate, documented deviation from the spec's literal
"expose `crew_plugin::broker::discover` provider discovery" instruction —
the discovery *decision logic* (`pick_provider`) is reused verbatim; only
the *construction* branches are duplicated in a new function, because nothing
in the codebase already returns a bare provider+model pair.

## Global Constraints

- **Trigger:** a cmdline starting with `!` is an AI ask; `bang_ask(line)` in
  `farpane/ask.rs` mirrors `crate::app::bang_command`/`star_command`'s
  `strip_prefix` + `trim` shape (`farpane::ask::bang_ask` is a *different*
  function from `crate::app::bang_command` — same prefix character, unrelated
  feature: the main bar's `!command` spawns a whole new pane running the
  literal command; the Far bar's `!` asks the AI for a command suggestion).
- **Enter submits the ask** (not `run_cmdline`) when `bang_ask(&p.cmdline)`
  is `Some` — checked in `keys.rs`'s `reduce()` before the existing
  `run_cmdline` fallback, exactly like Phase 1's `typing` gate.
- **`thinking…` status with elapsed seconds** is computed fresh every
  render frame from a stored `Instant` (`AskState::Thinking.started`) — no
  ticking timer state to maintain; `render.rs` calls `.elapsed().as_secs()`
  directly, the same "recompute from data, don't push updates" approach
  Phase 1 used for ghost text.
- **The suggestion REPLACES the bar's selected-style** — `command_bar`
  gains a `suggested: bool` that swaps the cmdline span's style to
  `fg(page_bg).bg(accent_color())`, the exact style the directory listing
  already uses for its active-cursor row (`render.rs`'s existing
  `highlight_style`) — and appends the hint `Enter run · Esc discard ·
  keep typing to edit` as a dim trailing span, the same position `running`
  already occupies.
- **Enter runs the suggestion via the normal `run_cmdline` path**: once
  landed, `cmdline` holds the bare suggested command (no `!` prefix), so
  Enter's existing `bang_ask` check returns `None` and falls through to
  `run_cmdline` unchanged — `p.history.push` records the **final command**,
  never the `!` ask text (the ask text never reaches `CmdHistory` at all).
- **Esc restores the `!` text**: `escape_cmdline` takes `AskState::Suggested
  { original }` and restores `p.cmdline = original` before falling through
  to the pre-existing cycle/clear/close chain.
- **Typing cancels the in-flight ask**: every cmdline-editing key arm
  (`Character`, `Space`, `Backspace`-while-typing) sets `p.ask = None`
  alongside the existing `p.complete = None` — dropping the `Receiver` so
  the worker thread's `tx.send` result is silently discarded (the thread
  itself still runs to completion in the background; only its result is
  ignored). The same reset also demotes a landed `Suggested` state to plain
  text once the user starts editing it ("keep typing to edit").
- **Errors land in the status line with the `!` text kept**: no-provider,
  timeout, and empty-reply all clear `p.ask` and leave `p.cmdline`
  untouched, returning a status string through the same `Option<String>`
  channel `poll_cmd` already uses.
- **20s timeout**: `ask::ASK_TIMEOUT = Duration::from_secs(20)`, passed
  through to `crew_plugin::suggest_far_command`'s `tokio::time::timeout`.
- **Spawned thread + `mpsc::Receiver` polled like `running`**: `submit_ask`
  in `run.rs` spawns exactly like `run::start` does; `FarPane::poll_ask`
  drains it exactly like `FarPane::poll_cmd` drains `running`, called from
  the same `crew-app/src/poll.rs` Far-pane arm.
- **`CREW_BROKER_MOCK_REPLY` short-circuits**: via `provider_and_model()` →
  `pick_provider()`, identical to every other broker entry point — no
  special-casing needed in `crew-app` at all.
- **`max_tokens: 128`**: `ask::FAR_MAX_TOKENS` in `crew-plugin`, verified by
  a direct unit test on the pure `far_request` builder (Task 1) rather than
  observed indirectly through the mock provider (which ignores
  `max_tokens` entirely).
- **Response post-processing strips fences/whitespace to one line**: reuses
  `extract_command` (already implemented, already tested in
  `ask_tests.rs`) — no new stripping logic.
- **Never auto-runs**: there is no code path from a landed suggestion to
  `run_cmdline` other than an explicit Enter keypress.
- **Zero `cargo check` warnings; rustfmt clean** for both crates touched
  (`crew-plugin`, `crew-app`).

---

### Task 1: `crew-plugin` — provider access + the far-ask call

**Files:**
- Modify: `crates/crew-plugin/src/broker/discover.rs` (add `provider_and_model`)
- Modify: `crates/crew-plugin/src/broker/ask.rs` (add `suggest_far_command`, `far_request`, `far_system_prompt`, `FAR_MAX_TOKENS`)
- Modify: `crates/crew-plugin/src/broker/ask_tests.rs` (append tests)
- Modify: `crates/crew-plugin/src/broker/mod.rs` (re-export `suggest_far_command`)
- Modify: `crates/crew-plugin/src/lib.rs` (re-export `suggest_far_command`)

**Interfaces:**
- Produces (consumed by Task 3's `run.rs`):
  - `pub fn suggest_far_command(query: &str, cwd: &std::path::Path, timeout: Duration) -> Result<String, String>`
- Internal (not consumed outside this crate):
  - `pub(crate) fn discover::provider_and_model() -> Option<(Arc<dyn crew_hive::Provider>, String)>`

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-plugin/src/broker/ask_tests.rs` (after the existing
tests, before the file ends):

```rust
#[test]
fn far_system_prompt_names_cwd_and_os_and_bans_prose() {
    let p = far_system_prompt(std::path::Path::new("/tmp/proj"));
    assert!(p.contains("/tmp/proj"), "cwd missing: {p}");
    assert!(p.contains(std::env::consts::OS), "os missing: {p}");
    let lower = p.to_lowercase();
    assert!(lower.contains("one") && lower.contains("command"));
    assert!(lower.contains("no prose"));
    assert!(lower.contains("no code fences") || lower.contains("no code fence"));
}

#[test]
fn far_request_caps_max_tokens_at_128_and_carries_the_system_prompt() {
    let req = far_request("list files", std::path::Path::new("/tmp"), "m".to_string());
    assert_eq!(req.max_tokens, 128);
    assert_eq!(req.prompt, "list files");
    assert_eq!(req.model, "m");
    assert!(req.system.unwrap().contains("/tmp"));
}

#[test]
fn mock_provider_answers_the_far_ask_and_strips_fences() {
    let _env = testenv::mock("```sh\nls -la\n```");
    let got = suggest_far_command(
        "list files",
        std::path::Path::new("/tmp"),
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(got, "ls -la");
}

#[test]
fn mock_provider_far_ask_survives_a_bare_reply_too() {
    let _env = testenv::mock("  du -sh *  \n");
    let got = suggest_far_command(
        "disk usage",
        std::path::Path::new("/tmp"),
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(got, "du -sh *");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-plugin broker::ask::`
Expected: compile FAIL — `far_system_prompt`, `far_request`,
`suggest_far_command` not found in this scope.

- [ ] **Step 3: Implement**

In `crates/crew-plugin/src/broker/discover.rs`, insert this new function
directly ABOVE the trailing `#[cfg(test)] #[path = "discover_tests.rs"] mod
tests;` block (i.e. right after `roster_with`'s closing brace):

```rust
/// Resolve the default provider + a single reasonable model id, without
/// building the full inbuilt-agent roster (`roster_with`) or its `Adapter`
/// wrapping. For one-shot low-token asks that need a custom system prompt
/// and a small `max_tokens` — neither of which the `Adapter` trait exposes
/// (`ApiAdapter::call` always sends the role's fixed system prompt and a
/// 2048-token ceiling). Used by the Far pane's `!` command suggestion
/// ([`super::ask::suggest_far_command`]). Mirrors `roster_with`'s branches
/// exactly (mock / DashScope / OpenRouter / Anthropic), picking
/// `ModelTier::Cheap` for Anthropic — a one-line shell suggestion needs no
/// deep reasoning — while DashScope/OpenRouter already default to their
/// cheapest usable chain head (`chain[0]`), so no tier mapping applies there.
pub(crate) fn provider_and_model() -> Option<(Arc<dyn crew_hive::Provider>, String)> {
    let force = std::env::var("CREW_PROVIDER").ok();
    let has = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    match pick_provider(force.as_deref(), has)? {
        ProviderKind::Mock => {
            let reply = std::env::var("CREW_BROKER_MOCK_REPLY").unwrap_or_default();
            let provider = crew_hive::MockProvider { reply };
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, "mock".to_string()))
        }
        ProviderKind::DashScope => {
            let key = std::env::var("DASHSCOPE_API_KEY").ok()?;
            let chain = parse_model_chain(
                std::env::var("CREW_DASHSCOPE_MODEL").ok(),
                DEFAULT_DASHSCOPE_CHAIN,
            );
            let url = std::env::var("CREW_DASHSCOPE_BASE_URL")
                .unwrap_or_else(|_| DASHSCOPE_ENDPOINT.to_string());
            let model = chain[0].clone();
            let provider = crew_hive::OpenRouterProvider::new(key)
                .with_endpoint(url)
                .with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::OpenRouter => {
            let provider = crew_hive::OpenRouterProvider::from_env().ok()?;
            let chain = parse_model_chain(
                std::env::var("CREW_OPENROUTER_MODEL").ok(),
                DEFAULT_OPENROUTER_CHAIN,
            );
            let model = chain[0].clone();
            let provider = provider.with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::Anthropic => {
            let provider = crew_hive::AnthropicProvider::from_env().ok()?;
            Some((
                Arc::new(provider) as Arc<dyn crew_hive::Provider>,
                crew_hive::ModelTier::Cheap.model_id().to_string(),
            ))
        }
    }
}
```

In `crates/crew-plugin/src/broker/ask.rs`, change the top of the file from:

```rust
//! One-shot "ask the AI for a command": powers the input bar's `?` prefix
//! (à la Warp AI / GitHub Copilot CLI). Reuses the broker's full provider
//! stack — mock, DashScope, OpenRouter, Anthropic, per-provider fallback
//! chains — via `discover::roster_with`, so the ask works wherever `/crew`'s
//! inbuilt agents do, with zero duplicated provider code. Blocking: call it
//! from a worker thread, never the render thread.
use std::time::Duration;
```

to:

```rust
//! One-shot "ask the AI for a command": powers the input bar's `?` prefix
//! (à la Warp AI / GitHub Copilot CLI) via `suggest_command`, and the Far
//! pane's `!` command bar suggestion via `suggest_far_command`. The former
//! reuses the broker's full `Adapter`/roster stack (`discover::roster_with`)
//! so it works wherever `/crew`'s inbuilt agents do; the latter calls
//! `discover::provider_and_model` directly, bypassing the `Adapter` layer,
//! because it needs a custom cwd/OS-aware system prompt and a small
//! `max_tokens` the roster's fixed-role adapters don't expose. Both are
//! blocking: call them from a worker thread, never the render thread.
use std::time::Duration;

use crew_hive::Provider;
```

Then insert this new code directly ABOVE the trailing `#[cfg(test)]
#[path = "ask_tests.rs"] mod tests;` line (i.e. after `extract_command`'s
closing brace):

```rust
/// Output-token ceiling for the Far pane's `!` one-shot ask: the model must
/// reply with a single command line, not a paragraph — small keeps latency
/// and cost down too.
const FAR_MAX_TOKENS: u32 = 128;

/// Translate `query` into exactly one POSIX shell command that will run in
/// `cwd` on this OS, via the discovered provider. Same discovery/mock rules
/// as [`suggest_command`] (`CREW_BROKER_MOCK_REPLY`, `CREW_PROVIDER`,
/// DASHSCOPE/OPENROUTER/ANTHROPIC auto-order, shell-env hydration) — but
/// calls the provider directly with a cwd/OS-aware system prompt and a small
/// `max_tokens`, bypassing the `Adapter`/roster layer entirely (see the
/// module doc comment for why). Used by the Far pane's `!` ask (crew-app's
/// `farpane` module); call from a worker thread, never the render thread.
pub fn suggest_far_command(
    query: &str,
    cwd: &std::path::Path,
    timeout: Duration,
) -> Result<String, String> {
    if std::env::var("CREW_BROKER_MOCK_REPLY").is_err() {
        static HYDRATE: std::sync::Once = std::sync::Once::new();
        HYDRATE.call_once(super::shellenv::hydrate);
    }
    let (provider, model) = super::discover::provider_and_model().ok_or_else(|| {
        "no AI provider — set DASHSCOPE_API_KEY, OPENROUTER_API_KEY, or ANTHROPIC_API_KEY"
            .to_string()
    })?;
    let req = far_request(query, cwd, model);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let fut = provider.complete(req);
    match rt.block_on(async move { tokio::time::timeout(timeout, fut).await }) {
        Ok(Ok(c)) => Ok(extract_command(&c.text)),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!("ask timed out after {timeout:?}")),
    }
}

/// Build the `!` ask's completion request: the exact system prompt, the
/// user's description as the prompt body, and the 128-token ceiling. Split
/// out from [`suggest_far_command`] so `max_tokens`/`system` are directly
/// unit-testable without a provider round-trip.
fn far_request(query: &str, cwd: &std::path::Path, model: String) -> crew_hive::CompletionRequest {
    crew_hive::CompletionRequest {
        model,
        system: Some(far_system_prompt(cwd)),
        prompt: query.to_string(),
        max_tokens: FAR_MAX_TOKENS,
    }
}

/// The exact system prompt the Far pane's `!` ask sends: demand one bare
/// POSIX command, name the directory it will run in and the OS.
fn far_system_prompt(cwd: &std::path::Path) -> String {
    format!(
        "Reply with exactly one POSIX shell command for the user's request. \
         No prose, no code fences. The command runs in {} on {}.",
        cwd.display(),
        std::env::consts::OS
    )
}
```

Then in `crates/crew-plugin/src/broker/mod.rs`, change:

```rust
pub use ask::{explain_output, suggest_command};
```

to:

```rust
pub use ask::{explain_output, suggest_command, suggest_far_command};
```

Then in `crates/crew-plugin/src/lib.rs`, change:

```rust
pub use broker::{
    explain_output, known_adapters, parse_routing, run_broker_stdio, suggest_command, Adapter,
};
```

to:

```rust
pub use broker::{
    explain_output, known_adapters, parse_routing, run_broker_stdio, suggest_command,
    suggest_far_command, Adapter,
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-plugin broker::`
Expected: PASS — all existing broker tests plus the 4 new `ask::` tests.

- [ ] **Step 5: Format + check clean**

Run: `cargo fmt -p crew-plugin` then `cargo check -p crew-plugin 2>&1 | grep -c warning` → `0`. Also run `cargo check -p crew-app 2>&1 | grep -c warning` → `0` (the new re-export must not break crew-app, which doesn't call it yet — this just confirms the crate still links).

- [ ] **Step 6: Commit**

```bash
git add crates/crew-plugin/src/broker/discover.rs crates/crew-plugin/src/broker/ask.rs crates/crew-plugin/src/broker/ask_tests.rs crates/crew-plugin/src/broker/mod.rs crates/crew-plugin/src/lib.rs
git commit -m "feat(crew): suggest_far_command — cwd/OS-aware one-shot ask, bypassing the Adapter roster"
```

---

### Task 2: `farpane/ask.rs` — the ask state machine + `FarPane` wiring

**Files:**
- Create: `crates/crew-app/src/farpane/ask.rs` (implementation + inline `#[cfg(test)] mod tests`)
- Modify: `crates/crew-app/src/farpane/mod.rs` (`mod ask;`, `FarPane.ask` field, `FarPane::new`, `poll_ask`/`absorb_ask_result` methods)
- Modify: `crates/crew-app/src/farpane/mod_tests.rs` (append; extend `use` block)

**Interfaces:**
- Produces (consumed by Task 3's `run.rs`/`keys.rs`, Task 4's `render.rs`):
  - `pub(crate) enum AskState { Thinking { started: Instant, rx: Receiver<Result<String, String>> }, Suggested { original: String } }`
  - `pub(crate) fn bang_ask(line: &str) -> Option<&str>`
  - `pub(crate) const ASK_TIMEOUT: Duration`
  - `FarPane.ask: Option<ask::AskState>`
  - `pub fn FarPane::poll_ask(&mut self) -> Option<String>`
- Consumes: nothing beyond `std`.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/farpane/ask.rs` with only this content:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bang_ask_parses_the_description() {
        assert_eq!(bang_ask("! list rust files"), Some("list rust files"));
        assert_eq!(bang_ask("!  kill port 8080 "), Some("kill port 8080"));
        assert_eq!(bang_ask("!"), Some(""));
        assert_eq!(bang_ask("!   "), Some(""));
    }

    #[test]
    fn lines_without_a_leading_bang_are_not_an_ask() {
        assert_eq!(bang_ask("ls -la"), None);
        assert_eq!(bang_ask("echo hi!"), None);
        assert_eq!(bang_ask(""), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::ask::`
Expected: compile FAIL — `bang_ask` not found in this scope.

- [ ] **Step 3: Implement**

Insert this ABOVE the `#[cfg(test)] mod tests { ... }` block:

```rust
//! The Far command bar's `!` AI ask: `! <description>` submits a one-shot
//! provider call (`crew_plugin::suggest_far_command`, Task 1) on a worker
//! thread; `FarPane::poll_ask` (this module's `mod.rs` half) drains the
//! result each tick, the same shape as `run.rs`'s `running`/`poll_cmd`, so
//! the winit thread never blocks on the network. The reply REPLACES the
//! bar's content as an editable, highlighted suggestion — Enter runs it via
//! the normal `run_cmdline` path, Esc restores the original `!` text, and
//! further typing just edits it like ordinary text. Distinct from
//! `crate::app`'s top-level `!command` (`bang_command`, spawns a whole
//! pane) — same prefix character, unrelated feature.
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Provider deadline for one ask — bounded so a dead network resolves to a
/// status line, not a forever-pending "thinking…".
pub(crate) const ASK_TIMEOUT: Duration = Duration::from_secs(20);

/// The Far pane's in-flight or landed `!` ask.
pub(crate) enum AskState {
    /// Waiting on the worker thread; `FarPane::cmdline` still shows the
    /// typed `! <description>` text untouched.
    Thinking {
        started: Instant,
        rx: Receiver<Result<String, String>>,
    },
    /// A suggestion landed and replaced `cmdline`; `original` is the `!
    /// <description>` text Esc restores.
    Suggested { original: String },
}

/// If `line` is a `! <description>` AI ask, return the trimmed description
/// (empty when just `!` or `!` followed only by whitespace); else `None`.
/// Checked in `keys.rs`'s Enter handling before falling back to
/// `run_cmdline`, the same explicit-prefix pattern as `crate::app`'s
/// `bang_command`/`star_command`.
pub(crate) fn bang_ask(line: &str) -> Option<&str> {
    line.strip_prefix('!').map(str::trim)
}
```

Then in `crates/crew-app/src/farpane/mod.rs`, change:

```rust
mod cmdhist;
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod render;
mod run;
```

to:

```rust
mod ask;
mod cmdhist;
mod complete;
mod fileops;
mod icons;
mod keys;
mod list;
mod render;
mod run;
```

Change the `FarPane` struct from:

```rust
pub struct FarPane {
    pub(crate) left: Panel,
    pub(crate) right: Panel,
    pub(crate) active: Side,
    /// Active text prompt (F7 make-folder), captured before any nav key.
    pub(crate) prompt: Option<Prompt>,
    /// The classic Far command line at the bottom: typed text runs (Enter) as a
    /// command in the active panel's directory. Empty when nothing is typed.
    pub(crate) cmdline: String,
    /// A command started from the command line that is still running on its
    /// worker thread: `(command text, result channel)`.
    pub(crate) running: Option<(String, std::sync::mpsc::Receiver<run::CmdDone>)>,
    /// Persisted command-line history (`far-history`) + Up/Down browse state
    /// and fish-style ghost-text lookups.
    pub(crate) history: cmdhist::CmdHistory,
    /// An in-progress Tab-completion cycle, if any — invalidated by any
    /// edit to `cmdline` (typing, Backspace, running a command).
    pub(crate) complete: Option<complete::CycleState>,
    /// Cached `$PATH` binaries for Command-kind Tab completion, filled by a
    /// background scan kicked off by the first Tab that needs it.
    pub(crate) bins: std::sync::Arc<std::sync::OnceLock<Vec<String>>>,
    /// Whether the `$PATH` scan thread has already been spawned — guards
    /// against spawning one per keystroke before the first scan lands.
    pub(crate) bins_scan_started: bool,
}
```

to:

```rust
pub struct FarPane {
    pub(crate) left: Panel,
    pub(crate) right: Panel,
    pub(crate) active: Side,
    /// Active text prompt (F7 make-folder), captured before any nav key.
    pub(crate) prompt: Option<Prompt>,
    /// The classic Far command line at the bottom: typed text runs (Enter) as a
    /// command in the active panel's directory. Empty when nothing is typed.
    pub(crate) cmdline: String,
    /// A command started from the command line that is still running on its
    /// worker thread: `(command text, result channel)`.
    pub(crate) running: Option<(String, std::sync::mpsc::Receiver<run::CmdDone>)>,
    /// Persisted command-line history (`far-history`) + Up/Down browse state
    /// and fish-style ghost-text lookups.
    pub(crate) history: cmdhist::CmdHistory,
    /// An in-progress Tab-completion cycle, if any — invalidated by any
    /// edit to `cmdline` (typing, Backspace, running a command).
    pub(crate) complete: Option<complete::CycleState>,
    /// Cached `$PATH` binaries for Command-kind Tab completion, filled by a
    /// background scan kicked off by the first Tab that needs it.
    pub(crate) bins: std::sync::Arc<std::sync::OnceLock<Vec<String>>>,
    /// Whether the `$PATH` scan thread has already been spawned — guards
    /// against spawning one per keystroke before the first scan lands.
    pub(crate) bins_scan_started: bool,
    /// The in-flight or landed `!` AI ask, if any — invalidated (`None`) by
    /// any edit to `cmdline`, same lifecycle rule as `complete`.
    pub(crate) ask: Option<ask::AskState>,
}
```

Change `FarPane::new` from:

```rust
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
            prompt: None,
            cmdline: String::new(),
            running: None,
            history: cmdhist::CmdHistory::load(),
            complete: None,
            bins: std::sync::Arc::new(std::sync::OnceLock::new()),
            bins_scan_started: false,
        }
    }
```

to:

```rust
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            left: Panel::new(cwd.clone()),
            right: Panel::new(cwd),
            active: Side::Left,
            prompt: None,
            cmdline: String::new(),
            running: None,
            history: cmdhist::CmdHistory::load(),
            complete: None,
            bins: std::sync::Arc::new(std::sync::OnceLock::new()),
            bins_scan_started: false,
            ask: None,
        }
    }
```

Add two new methods to `impl FarPane`, directly below `poll_cmd`:

```rust
    /// Drain a finished `!` ask, if any: land it (via [`Self::absorb_ask_result`])
    /// or report the worker thread dying without a reply. Returns a status
    /// line for the app to flash, mirroring `poll_cmd`; `None` when nothing
    /// changed this tick (still thinking, or no ask at all).
    pub fn poll_ask(&mut self) -> Option<String> {
        let Some(ask::AskState::Thinking { rx, .. }) = &self.ask else {
            return None;
        };
        match rx.try_recv() {
            Ok(res) => Some(self.absorb_ask_result(res)),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.ask = None;
                Some("ask failed: worker died — ! text kept".to_string())
            }
        }
    }

    /// Land a finished ask's result: a non-blank suggestion replaces
    /// `cmdline` (state becomes `Suggested`, `original` keeps the `!` text
    /// for Esc); a blank suggestion or an error clears `ask` and leaves
    /// `cmdline` untouched. Returns the status line either way.
    fn absorb_ask_result(&mut self, res: Result<String, String>) -> String {
        match res {
            Ok(cmd) if cmd.trim().is_empty() => {
                self.ask = None;
                "no command suggested — ! text kept".to_string()
            }
            Ok(cmd) => {
                let original = std::mem::replace(&mut self.cmdline, cmd.trim().to_string());
                self.ask = Some(ask::AskState::Suggested { original });
                "Enter run \u{b7} Esc discard \u{b7} keep typing to edit".to_string()
            }
            Err(e) => {
                self.ask = None;
                format!("ask failed: {e} — ! text kept")
            }
        }
    }
```

Note: `\u{b7}` is the middle-dot `·` — written as an escape here so the
plan's own markdown code fence can't mangle the literal character; the
actual source file may use either form (both compile identically).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::`
Expected: PASS — all Phase-1 tests plus 2 new `ask::` tests.

- [ ] **Step 5: Add `FarPane`-level tests**

Extend the `use` block at the top of `crates/crew-app/src/farpane/mod_tests.rs` from:

```rust
use super::cmdhist::CmdHistory;
use super::keys::{
    accept_ghost, activate, ascend, escape_cmdline, history_next, history_prev, move_sel,
    tab_complete,
};
use super::run::run_cmdline;
use super::{FarAction, FarPane, Side};
```

to:

```rust
use super::ask::AskState;
use super::cmdhist::CmdHistory;
use super::keys::{
    accept_ghost, activate, ascend, escape_cmdline, history_next, history_prev, move_sel,
    tab_complete,
};
use super::run::run_cmdline;
use super::{FarAction, FarPane, Side};
```

Append these tests (after the existing tests, before the closing brace):

```rust
#[test]
fn new_pane_starts_with_no_ask() {
    let (_b, p) = fixture("noask");
    assert!(p.ask.is_none());
}

#[test]
fn absorb_ask_result_lands_a_suggestion_and_replaces_the_bar() {
    let (_b, mut p) = fixture("askland");
    p.cmdline = "! list files".into();
    let msg = p.absorb_ask_result(Ok("ls -la".into()));
    assert_eq!(p.cmdline, "ls -la");
    assert!(
        matches!(&p.ask, Some(AskState::Suggested { original }) if original == "! list files")
    );
    assert!(msg.contains("Enter run"));
}

#[test]
fn absorb_ask_result_treats_a_blank_suggestion_as_no_command() {
    let (_b, mut p) = fixture("askblank");
    p.cmdline = "! list files".into();
    let msg = p.absorb_ask_result(Ok("   ".into()));
    assert_eq!(p.cmdline, "! list files", "the ! text is kept on an empty reply");
    assert!(p.ask.is_none());
    assert!(msg.contains("no command"));
}

#[test]
fn absorb_ask_result_surfaces_a_provider_error_and_keeps_the_bang_text() {
    let (_b, mut p) = fixture("askerr");
    p.cmdline = "! list files".into();
    let msg = p.absorb_ask_result(Err("no AI provider".into()));
    assert_eq!(p.cmdline, "! list files");
    assert!(p.ask.is_none());
    assert!(msg.contains("no AI provider"));
}

#[test]
fn poll_ask_returns_none_while_still_thinking() {
    let (_b, mut p) = fixture("askthinking");
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    assert!(p.poll_ask().is_none());
    assert!(p.ask.is_some(), "still thinking — ask state untouched");
}

#[test]
fn poll_ask_drains_a_landed_result_via_absorb() {
    let (_b, mut p) = fixture("askdrain");
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    tx.send(Ok("ls -la".into())).unwrap();
    p.cmdline = "! list files".into();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    let msg = p.poll_ask();
    assert_eq!(p.cmdline, "ls -la");
    assert!(msg.unwrap().contains("Enter run"));
}

#[test]
fn poll_ask_handles_a_dead_worker_thread() {
    let (_b, mut p) = fixture("askdead");
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    drop(tx); // disconnect without sending — worker panicked/died
    p.cmdline = "! list files".into();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    let msg = p.poll_ask();
    assert_eq!(p.cmdline, "! list files");
    assert!(p.ask.is_none());
    assert!(msg.unwrap().contains("worker died"));
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::`
Expected: PASS — Phase-1 tests + 2 `ask::` unit tests + 7 new `mod_tests.rs` tests.

- [ ] **Step 7: Format + check clean**

Run: `cargo fmt -p crew-app` then `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 8: Commit**

```bash
git add crates/crew-app/src/farpane/ask.rs crates/crew-app/src/farpane/mod.rs crates/crew-app/src/farpane/mod_tests.rs
git commit -m "feat(crew): FarPane ask state machine — AskState, poll_ask, absorb_ask_result"
```

---

### Task 3: Wire keys — Enter submits, Esc discards, typing cancels

**Files:**
- Modify: `crates/crew-app/src/farpane/run.rs` (new `submit_ask`; `run_cmdline` clears `p.ask`)
- Modify: `crates/crew-app/src/farpane/keys.rs` (Enter/Backspace/Space/Character arms; `escape_cmdline`)
- Modify: `crates/crew-app/src/farpane/ask.rs` (add `test_guard` for the one env-mutating test)
- Modify: `crates/crew-app/src/farpane/mod_tests.rs` (append)

**Interfaces:**
- Consumes: `ask::{AskState, ASK_TIMEOUT, bang_ask}` (Task 2); `crew_plugin::suggest_far_command` (Task 1).
- Produces (new `pub(crate)` function in `run.rs`, callable directly from tests like `run_cmdline` already is):
  - `pub(crate) fn submit_ask(p: &mut FarPane, desc: &str) -> FarAction`

- [ ] **Step 1: Write the failing tests**

First, extend `crates/crew-app/src/farpane/mod_tests.rs`'s `use` block from:

```rust
use super::ask::AskState;
use super::cmdhist::CmdHistory;
use super::keys::{
    accept_ghost, activate, ascend, escape_cmdline, history_next, history_prev, move_sel,
    tab_complete,
};
use super::run::run_cmdline;
use super::{FarAction, FarPane, Side};
```

to:

```rust
use super::ask::AskState;
use super::cmdhist::CmdHistory;
use super::keys::{
    accept_ghost, activate, ascend, escape_cmdline, history_next, history_prev, move_sel,
    tab_complete,
};
use super::run::{run_cmdline, submit_ask};
use super::{FarAction, FarPane, Side};
```

Then append these tests (after Task 2's tests, before the closing brace):

```rust
#[test]
fn submit_ask_starts_thinking_and_keeps_the_bang_text() {
    let (_b, mut p) = fixture("bangenter");
    p.cmdline = "! list files".into();
    let action = submit_ask(&mut p, "list files");
    assert!(matches!(action, FarAction::Status(ref s) if s.contains("asking ai")));
    assert!(matches!(p.ask, Some(AskState::Thinking { .. })));
    assert_eq!(p.cmdline, "! list files", "the ! text stays while thinking");
}

#[test]
fn submit_ask_nags_on_a_blank_description() {
    let (_b, mut p) = fixture("bangblank");
    let action = submit_ask(&mut p, "");
    assert!(matches!(action, FarAction::Status(ref s) if s.contains("description")));
    assert!(p.ask.is_none());
}

#[test]
fn submit_ask_refuses_a_second_ask_while_one_is_in_flight() {
    let (_b, mut p) = fixture("bangbusy");
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    let action = submit_ask(&mut p, "another one");
    assert!(matches!(action, FarAction::Status(ref s) if s.contains("wait")));
}

#[test]
fn escape_on_a_suggestion_restores_the_original_bang_text() {
    let (_b, mut p) = fixture("bangesc");
    p.cmdline = "ls -la".into();
    p.ask = Some(AskState::Suggested {
        original: "! list files".into(),
    });
    assert!(escape_cmdline(&mut p).is_none());
    assert_eq!(p.cmdline, "! list files");
    assert!(p.ask.is_none());
}

#[test]
fn escape_while_thinking_cancels_the_ask_and_clears_the_bar() {
    let (_b, mut p) = fixture("bangescthink");
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    p.cmdline = "! list files".into();
    p.ask = Some(AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    assert!(escape_cmdline(&mut p).is_none());
    assert!(p.ask.is_none());
    assert!(p.cmdline.is_empty(), "Esc's normal non-empty-bar clear still applies");
}

#[test]
fn run_cmdline_after_accepting_a_suggestion_clears_the_ask_state() {
    let _g = super::ask::test_guard();
    with_tmp_home(|| {
        let (base, mut p) = fixture("bangaccept");
        p.right.cwd = base.join("sub");
        p.active = Side::Right;
        p.cmdline = "touch made-here".into();
        p.ask = Some(AskState::Suggested {
            original: "! make a file".into(),
        });
        run_cmdline(&mut p);
        assert!(p.ask.is_none());
        assert_eq!(p.history.prev(""), Some("touch made-here"), "history records the final command, not the ! ask");
    });
}

#[test]
fn bang_ask_end_to_end_with_the_mock_provider() {
    let _g = super::ask::test_guard();
    std::env::set_var("CREW_BROKER_MOCK_REPLY", "ls -la");
    let (_b, mut p) = fixture("bange2e");
    p.cmdline = "! list files".into();
    submit_ask(&mut p, "list files");
    let mut landed = None;
    for _ in 0..300 {
        if let Some(msg) = p.poll_ask() {
            landed = Some(msg);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::env::remove_var("CREW_BROKER_MOCK_REPLY");
    assert!(landed.unwrap().contains("Enter run"), "the hint reaches the caller");
    assert_eq!(p.cmdline, "ls -la");
    assert!(
        matches!(&p.ask, Some(AskState::Suggested { original }) if original == "! list files")
    );
}
```

Note: `run_cmdline_after_accepting_a_suggestion_clears_the_ask_state` and
`bang_ask_end_to_end_with_the_mock_provider` reuse the file's existing
`with_tmp_home` helper (added by Phase 1's Task 4, already present in this
file) and the `super::ask::test_guard()` this task adds below — both guard
against real filesystem/env-var mutation racing other parallel tests,
mirroring `cmdhist::test_guard`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::`
Expected: compile FAIL — `submit_ask` not found in `run.rs`, `test_guard`
not found in `ask.rs`.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/farpane/ask.rs`, append this at the very end of the
file (after `bang_ask`, still above the `#[cfg(test)] mod tests` block):

```rust
/// Serialises tests that mutate `CREW_BROKER_MOCK_REPLY` — several tests
/// spawn a real worker thread that reads this env var via
/// `crew_plugin::suggest_far_command` and would otherwise race under the
/// default parallel test runner. Mirrors `cmdhist::test_guard`.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
```

In `crates/crew-app/src/farpane/run.rs`, change the top of the file from:

```rust
use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use super::keys::FarAction;
use super::FarPane;
```

to:

```rust
use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use super::ask;
use super::keys::FarAction;
use super::FarPane;
```

Change `run_cmdline` from:

```rust
pub(crate) fn run_cmdline(p: &mut FarPane) -> FarAction {
    let cwd = p.active_cwd();
    let cmd = std::mem::take(&mut p.cmdline);
    let cmd = cmd.trim().to_string();
    p.complete = None;
    if cmd.is_empty() {
        return FarAction::Status("nothing to run".into());
    }
```

to:

```rust
pub(crate) fn run_cmdline(p: &mut FarPane) -> FarAction {
    let cwd = p.active_cwd();
    let cmd = std::mem::take(&mut p.cmdline);
    let cmd = cmd.trim().to_string();
    p.complete = None;
    p.ask = None;
    if cmd.is_empty() {
        return FarAction::Status("nothing to run".into());
    }
```

Add this new function directly below `run_cmdline` (before `change_dir`):

```rust
/// Submit a `! <description>` ask on a worker thread: `crew_plugin`'s
/// discovered provider translates it into one shell command for the active
/// panel's directory (never auto-run — the reply lands via `poll_ask` as an
/// editable, highlighted suggestion). Refused, with no thread spawned, while
/// another ask is already in flight or when `desc` is blank.
pub(crate) fn submit_ask(p: &mut FarPane, desc: &str) -> FarAction {
    if p.ask.is_some() {
        return FarAction::Status("still asking — wait for it".into());
    }
    if desc.is_empty() {
        return FarAction::Status("type a description after !".into());
    }
    let cwd = p.active_cwd();
    let query = desc.to_string();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(crew_plugin::suggest_far_command(&query, &cwd, ask::ASK_TIMEOUT));
    });
    p.ask = Some(ask::AskState::Thinking {
        started: std::time::Instant::now(),
        rx,
    });
    FarAction::Status(format!("asking ai — {desc}"))
}
```

In `crates/crew-app/src/farpane/keys.rs`, change the Enter arm from:

```rust
        // Enter runs a typed command; with an empty command line it activates
        // the selected entry (descend / open), preserving the old behaviour.
        Key::Named(NamedKey::Enter) => {
            if typing {
                return Some(run_cmdline(p));
            }
            return activate(p);
        }
```

to:

```rust
        // Enter runs a typed command, submits a `!` ask, or (empty bar)
        // activates the selected entry (descend / open). A landed
        // suggestion's text never starts with `!` (it's the bare command),
        // so this falls straight through to `run_cmdline` on accept.
        Key::Named(NamedKey::Enter) => {
            if typing {
                if let Some(desc) = super::ask::bang_ask(&p.cmdline) {
                    return Some(super::run::submit_ask(p, desc));
                }
                return Some(run_cmdline(p));
            }
            return activate(p);
        }
```

Change the Backspace arm from:

```rust
        // Backspace edits the command line while typing, else ascends.
        Key::Named(NamedKey::Backspace) => {
            if typing {
                p.cmdline.pop();
                p.complete = None;
            } else {
                ascend(p);
            }
        }
```

to:

```rust
        // Backspace edits the command line while typing, else ascends.
        Key::Named(NamedKey::Backspace) => {
            if typing {
                p.cmdline.pop();
                p.complete = None;
                p.ask = None;
            } else {
                ascend(p);
            }
        }
```

Change the Space/Character arms from:

```rust
        // Printable input builds up the command line (classic Far behaviour).
        Key::Named(NamedKey::Space) => {
            p.cmdline.push(' ');
            p.complete = None;
        }
        Key::Character(s) => {
            p.cmdline.push_str(s.as_str());
            p.complete = None;
        }
```

to:

```rust
        // Printable input builds up the command line (classic Far
        // behaviour); any edit cancels an in-flight `!` ask (the worker
        // thread still finishes in the background, but its result is now
        // dropped — see `FarPane::poll_ask`) and demotes a landed
        // suggestion back to plain, unhighlighted text ("keep typing to
        // edit").
        Key::Named(NamedKey::Space) => {
            p.cmdline.push(' ');
            p.complete = None;
            p.ask = None;
        }
        Key::Character(s) => {
            p.cmdline.push_str(s.as_str());
            p.complete = None;
            p.ask = None;
        }
```

Change `escape_cmdline` from:

```rust
pub(crate) fn escape_cmdline(p: &mut FarPane) -> Option<FarAction> {
    if let Some(state) = p.complete.take() {
        p.cmdline = state.prefix;
        return None;
    }
    if !p.cmdline.is_empty() {
        p.cmdline.clear();
        return None;
    }
    Some(FarAction::Close)
}
```

to:

```rust
pub(crate) fn escape_cmdline(p: &mut FarPane) -> Option<FarAction> {
    if let Some(state) = p.complete.take() {
        p.cmdline = state.prefix;
        return None;
    }
    // A landed suggestion restores the original `!` text verbatim and
    // discards the suggestion. A still-thinking ask just cancels (its
    // worker thread finishes in the background but the result is dropped)
    // and falls through to the normal clear/close behaviour below.
    if let Some(super::ask::AskState::Suggested { original }) = p.ask.take() {
        p.cmdline = original;
        return None;
    }
    if !p.cmdline.is_empty() {
        p.cmdline.clear();
        return None;
    }
    Some(FarAction::Close)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::`
Expected: PASS — all prior tests plus the 6 new tests in this task.
`bang_ask_end_to_end_with_the_mock_provider` polls with up to 3s of sleep
budget (300 × 10ms) — generous for the mock provider, which resolves
near-instantly on a real thread.

- [ ] **Step 5: Format + check clean**

Run: `cargo fmt -p crew-app` then `cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/ask.rs crates/crew-app/src/farpane/run.rs crates/crew-app/src/farpane/keys.rs crates/crew-app/src/farpane/mod_tests.rs
git commit -m "feat(crew): far cmdbar — ! submits an AI ask, Esc discards, typing cancels"
```

---

### Task 4: Render the ask status + wire the app poll loop

**Files:**
- Modify: `crates/crew-app/src/farpane/render.rs` (`render()` computes the ask hint; `command_bar` gains `ask_hint`/`suggested` params)
- Modify: `crates/crew-app/src/farpane/render_tests.rs` (append)
- Modify: `crates/crew-app/src/poll.rs` (Far-pane arm also polls `poll_ask`)

**Interfaces:**
- Consumes: `FarPane.ask` (Task 2); `ask::AskState` (Task 2); `crate::palette::accent_color`/`accent` (existing).
- Produces: nothing new outside `render.rs` (rendering is a leaf); `poll.rs`'s change has no new public surface.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-app/src/farpane/render_tests.rs` (after the existing
tests, before the closing brace):

```rust
#[test]
fn thinking_status_shows_elapsed_seconds() {
    use crate::farpane::ask::AskState;
    let mut pane = fixture_pane("thinking");
    pane.cmdline = "! list files".into();
    let (_tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    pane.ask = Some(AskState::Thinking {
        started: std::time::Instant::now() - std::time::Duration::from_secs(3),
        rx,
    });
    let cells = render(&pane, 80, 24);
    let cmd_row = 22; // rows(24) - cmdline row(1) - function bar row(1)
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    assert!(line.contains("thinking"), "missing thinking status: {line:?}");
    assert!(line.contains('3'), "elapsed seconds missing: {line:?}");
}

#[test]
fn suggested_command_highlights_the_bar_and_shows_the_accept_hint() {
    use crate::farpane::ask::AskState;
    let _g = crate::palette::test_guard();
    let mut pane = fixture_pane("suggested");
    pane.cmdline = "ls -la".into();
    pane.ask = Some(AskState::Suggested {
        original: "! list files".into(),
    });
    let cells = render(&pane, 80, 24);
    let cmd_row = 22;
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.iter().map(|(_, c)| *c).collect();
    assert!(line.contains("ls -la"), "suggestion missing: {line:?}");
    assert!(line.contains("Enter run"), "accept hint missing: {line:?}");
    let dash = cells
        .iter()
        .find(|c| c.row == cmd_row && c.c == '-')
        .expect("suggestion cell rendered");
    assert_eq!(
        dash.bg,
        crate::palette::accent(),
        "a landed suggestion highlights with the accent fill"
    );
}

#[test]
fn no_ask_status_when_ask_is_absent() {
    let mut pane = fixture_pane("noask");
    pane.cmdline = "ls".into();
    let cells = render(&pane, 80, 24);
    let cmd_row = 22;
    let line: String = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| c.c)
        .collect();
    assert!(!line.contains("thinking"));
    assert!(!line.contains("Enter run"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app farpane::render::`
Expected: the file FAILS TO COMPILE at first (`command_bar` doesn't yet
take `ask_hint`/`suggested`, so `render()`'s existing call site needs Step
3 too) — this is expected; the test file and the implementation land
together in Step 3, matching Phase 1's Task 5 precedent for a widened
private helper signature.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/farpane/render.rs`, change the tail of `render()`
from:

```rust
    scroll_thumb(&mut buf, larea, &p.left, p.active == Side::Left);
    scroll_thumb(&mut buf, rarea, &p.right, p.active == Side::Right);
    // A Tab-cycle already shows its candidate in `cmdline` directly; the
    // ghost suggestion would be confusing layered on top of it, so it's
    // suppressed while a cycle is active.
    let ghost = if p.complete.is_none() {
        p.history
            .ghost(&p.cmdline)
            .map(|full| full[p.cmdline.len()..].to_string())
    } else {
        None
    };
    let running = p.running.as_ref().map(|(cmd, _)| cmd.as_str());
    command_bar(
        &mut buf,
        split[1],
        &p.active_cwd(),
        &p.cmdline,
        ghost.as_deref(),
        running,
    );
```

to:

```rust
    scroll_thumb(&mut buf, larea, &p.left, p.active == Side::Left);
    scroll_thumb(&mut buf, rarea, &p.right, p.active == Side::Right);
    // A Tab-cycle already shows its candidate in `cmdline` directly; the
    // ghost suggestion would be confusing layered on top of it, so it's
    // suppressed while a cycle is active.
    let ghost = if p.complete.is_none() {
        p.history
            .ghost(&p.cmdline)
            .map(|full| full[p.cmdline.len()..].to_string())
    } else {
        None
    };
    // The `!` ask's live status: elapsed seconds while thinking (recomputed
    // fresh every frame from the stored `Instant` — nothing to tick), or
    // the accept/discard/edit hint once a suggestion has landed.
    let (ask_hint, suggested) = match &p.ask {
        Some(super::ask::AskState::Thinking { started, .. }) => (
            Some(format!("thinking\u{2026} {}s", started.elapsed().as_secs())),
            false,
        ),
        Some(super::ask::AskState::Suggested { .. }) => (
            Some("Enter run \u{b7} Esc discard \u{b7} keep typing to edit".to_string()),
            true,
        ),
        None => (None, false),
    };
    let running = p.running.as_ref().map(|(cmd, _)| cmd.as_str());
    command_bar(
        &mut buf,
        split[1],
        &p.active_cwd(),
        &p.cmdline,
        ghost.as_deref(),
        ask_hint.as_deref(),
        suggested,
        running,
    );
```

And change `command_bar` from:

```rust
fn command_bar(
    buf: &mut Buffer,
    area: Rect,
    cwd: &std::path::Path,
    cmdline: &str,
    ghost: Option<&str>,
    running: Option<&str>,
) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let folder = cwd
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    let mut spans = vec![
        Span::styled(format!("{folder} "), Style::new().fg(dim).bg(bg)),
        Span::styled("$ ", Style::new().fg(accent_color()).bg(bg)),
        Span::styled(format!("{cmdline}▏"), Style::new().fg(ink).bg(bg)),
    ];
    if let Some(g) = ghost {
        spans.push(Span::styled(g.to_string(), Style::new().fg(dim).bg(bg)));
    }
    if let Some(cmd) = running {
        spans.push(Span::styled(
            format!("  \u{27f3} {cmd}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bg))
        .render(area, buf);
}
```

to:

```rust
fn command_bar(
    buf: &mut Buffer,
    area: Rect,
    cwd: &std::path::Path,
    cmdline: &str,
    ghost: Option<&str>,
    ask_hint: Option<&str>,
    suggested: bool,
    running: Option<&str>,
) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let page = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let folder = cwd
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    // A landed `!` suggestion REPLACES the bar's normal styling with the
    // same selected look the panel listing uses for its cursor row (ink on
    // an accent fill) — a highlighted, still-editable suggestion.
    let cmd_style = if suggested {
        Style::new().fg(page).bg(accent_color())
    } else {
        Style::new().fg(ink).bg(bg)
    };
    let mut spans = vec![
        Span::styled(format!("{folder} "), Style::new().fg(dim).bg(bg)),
        Span::styled("$ ", Style::new().fg(accent_color()).bg(bg)),
        Span::styled(format!("{cmdline}▏"), cmd_style),
    ];
    if let Some(g) = ghost {
        spans.push(Span::styled(g.to_string(), Style::new().fg(dim).bg(bg)));
    }
    if let Some(hint) = ask_hint {
        spans.push(Span::styled(format!("  {hint}"), Style::new().fg(dim).bg(bg)));
    }
    if let Some(cmd) = running {
        spans.push(Span::styled(
            format!("  \u{27f3} {cmd}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bg))
        .render(area, buf);
}
```

In `crates/crew-app/src/poll.rs`, change the Far-pane arm from:

```rust
                // A Far pane changes when its command-line command finishes
                // (panels reload; the result becomes a status flash).
                PaneContent::Far(f) => match f.poll_cmd() {
                    Some(msg) => {
                        far_statuses.push(msg);
                        true
                    }
                    None => false,
                },
```

to:

```rust
                // A Far pane changes when its command-line command finishes
                // (panels reload) or its `!` AI ask lands/errors — either
                // becomes a status flash.
                PaneContent::Far(f) => {
                    let mut changed = false;
                    if let Some(msg) = f.poll_cmd() {
                        far_statuses.push(msg);
                        changed = true;
                    }
                    if let Some(msg) = f.poll_ask() {
                        far_statuses.push(msg);
                        changed = true;
                    }
                    changed
                }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app farpane::render::`
Expected: PASS (Phase-1's 17 render tests + 3 new).

- [ ] **Step 5: Full-suite run + format + check clean**

Run: `cargo test -p crew-app` → PASS (every suite, both crates'
`suggest_far_command`/`FarPane` ask lifecycle included). Then
`cargo fmt -p crew-app` and
`cargo check -p crew-app --bin crew 2>&1 | grep -c warning` → `0`. There is
no dedicated test for the `poll.rs` change — `poll_panes` requires a live
`winit` window (`if self.window.is_none() { return; }`) and the file has no
`#[cfg(test)]` module today; this mirrors the file's existing untested
status for the parallel `poll_cmd` line it sits beside.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/farpane/render.rs crates/crew-app/src/farpane/render_tests.rs crates/crew-app/src/poll.rs
git commit -m "feat(crew): far cmdbar — render the ! ask's thinking/suggested status, wire app poll loop"
```

---

## Self-Review Notes

**Spec coverage mapping** (`docs/superpowers/specs/2026-07-11-far-cmdbar-ai-design.md`):
- Trigger/Enter-submits/typing-cancels/errors-keep-text → Task 3 (`bang_ask` check in Enter, `p.ask = None` in every edit arm, `absorb_ask_result`'s error branch).
- `thinking…` with elapsed seconds → Task 4 (`render()` computes `started.elapsed().as_secs()` fresh every frame — no ticking state anywhere).
- Suggestion replaces the bar, selected-style, hint text, Esc restores → Task 2 (`AskState::Suggested`, `absorb_ask_result`) + Task 3 (`escape_cmdline`) + Task 4 (`command_bar`'s `suggested`/`ask_hint` params).
- Enter runs via normal `run_cmdline` (history records the final command, not the ask) → Task 3's Enter arm (falls through when `bang_ask` returns `None`, true once `cmdline` holds the bare suggestion) + the `run_cmdline_after_accepting_a_suggestion_clears_the_ask_state` test, which asserts `p.history.prev("") == Some("touch made-here")`, never the `!` text.
- Provider access without a broker child, `CREW_BROKER_MOCK_REPLY` short-circuit, `max_tokens: 128`, cwd/OS system prompt, fence/whitespace post-processing → Task 1 (`provider_and_model`, `suggest_far_command`, `far_request`, reused `extract_command`).
- 20s timeout, spawned thread + `mpsc::Receiver` polled like `running` → Task 2 (`ASK_TIMEOUT`, `AskState::Thinking.rx`) + Task 3 (`submit_ask`) + Task 4 (`poll.rs`'s new `f.poll_ask()` line, same shape as `f.poll_cmd()`).
- Never auto-runs → verified structurally: no code path reaches `run_cmdline` except the Enter key arm; `absorb_ask_result` only ever writes to `cmdline`/`ask`, never spawns a command.

**Provider-plumbing decision** (the task's central ambiguity, resolved up
front in its own section above): the existing `?`/`??` ask plumbing
(`suggest_command`/`explain_output`, in-process via `discover::roster_with`
→ `Adapter::call`) was investigated and found unsuitable to reuse
byte-for-byte, because `Adapter::call` hardcodes a role's system prompt and
a 2048-token ceiling — both of which the spec's `!` ask needs to override
per-call. The chosen design reuses everything reusable (`pick_provider`'s
decision logic, `extract_command`'s post-processing, the exact
worker-thread + `mpsc::Receiver` + poll-every-tick shape `?` already
established) and adds only the minimum new surface: one function that
returns a raw `(Provider, model)` pair instead of a role-wrapped `Adapter`.

**Type-consistency check:**
- `AskState::Thinking { started: Instant, rx: Receiver<Result<String, String>> }` — the `rx` type matches `submit_ask`'s channel (`mpsc::channel()` inferred from `tx.send(crew_plugin::suggest_far_command(...))`, whose return type is exactly `Result<String, String>`) and `crew_plugin::suggest_far_command`'s declared return type (Task 1). No adapter/conversion needed anywhere in the chain.
- `FarPane.ask: Option<ask::AskState>` is reset to `None` in exactly four places — `run_cmdline` (Task 2), the three cmdline-editing key arms (Task 3) — mirroring `p.complete`'s existing four reset sites from Phase 1 exactly (same file, same arms, same rationale comment pattern).
- `command_bar`'s new `(ghost, ask_hint, suggested, running)` parameter order matches every call site (`render()`'s single call, Task 4) and every test's expectations (`fixture_pane` + direct `AskState` construction, no `command_bar` unit tests needed since it's a private leaf function already covered via `render()`).
- `provider_and_model() -> Option<(Arc<dyn crew_hive::Provider>, String)>` and `far_request(query, cwd, model: String) -> CompletionRequest` compose without casts beyond the `as Arc<dyn crew_hive::Provider>` needed once per match arm in `provider_and_model` (necessary because each arm constructs a different concrete provider type; the function's declared return type drives the coercion).

**Resolved ambiguities** (spec left these implicit or explicitly asked this
plan to resolve; decisions made so the plan has no "TBD"):
1. **Provider plumbing** — covered in its own section above; the single most consequential decision in this plan.
2. **Where "the status line" renders.** The spec says "status line shows `thinking…`" and the suggestion's hint. `crew-app` has two different status mechanisms: a global 3-second-TTL flash (`CrewApp::set_status`/`active_status`, used for one-off toasts) and per-pane persistent inline rendering (`command_bar`'s existing `running: Option<&str>` "⟳ cmd" segment, recomputed every frame). A `thinking…` counter that must tick for up to 20s, and a suggestion hint that must stay visible for as long as the user takes to read it, cannot use the 3s-TTL flash without disappearing mid-read — so both render inline in `command_bar`, the same mechanism `running` already uses. `submit_ask`'s one-shot "asking ai — …" acknowledgment and every error message DO go through the transient `FarAction::Status`/`poll_ask`-returned-`String` flash (via `crew-app/src/keys.rs`'s existing `FarAction::Status(msg) => self.set_status(&msg)`, unmodified), since those are genuinely one-off events, not persistent state.
3. **What Esc does while still thinking (not yet covered by the spec's literal "Esc restores the `!` text," which only describes the landed-suggestion case).** Resolved as: cancel the ask (drop the receiver) and fall through to the pre-existing "clear if non-empty, else close" behaviour — consistent with how `escape_cmdline` already treats an active Tab-cycle (Phase 1) as one tier above the base clear/close chain, not a wholesale replacement of it.
4. **What typing over a landed suggestion does (the spec's "keep typing to edit" bullet, read as one of three peer options alongside Enter/Esc).** Resolved as: the first edit keystroke clears `p.ask` entirely (same reset as canceling a `Thinking` ask), demoting the suggestion to plain, unhighlighted, un-restorable-by-Esc text — matching the file's existing "any edit invalidates in-progress cmdline state" convention for `p.complete`.
5. **Anthropic's model tier for `provider_and_model`.** The spec doesn't name one; `ModelTier::Cheap` (`claude-haiku-4-5`) was chosen because a one-line shell suggestion needs no deep reasoning, and it parallels DashScope/OpenRouter already defaulting to their chain's cheapest-first entry (`chain[0]`) for this same call.
6. **Runtime construction per ask (not cached).** `ApiAdapter` caches a `tokio::runtime::Runtime` at construction because it's a persistent adapter reused across many broker relay calls. `suggest_far_command` builds a fresh current-thread runtime per call instead — asks are infrequent one-shots with no adapter object to cache it on, and `tokio::runtime::Builder::new_current_thread().build()` is cheap enough (same justification implicitly already accepted by `ApiAdapter::new`'s own doc comment, just applied per-call instead of per-adapter-lifetime here).
7. **Ghost text during a landed suggestion.** Phase 1's ghost text is only suppressed during an active Tab-cycle (`p.complete.is_some()`); it is NOT suppressed while `p.ask` is `Suggested`. Left as-is (YAGNI) — the spec doesn't mention an interaction between history ghost-text and AI suggestions, and the two rendering rules (`suggested`'s highlighted cmdline span, `ghost`'s dim trailing span) don't visually conflict since ghost text only ever appears strictly after the cmdline's own cursor bar.
8. **`reduce()`'s own dispatch stays untested**, same limitation Phase 1 documented (`winit::event::KeyEvent` is `#[non_exhaustive]`) — every new behavior added to `reduce()`'s match arms in Task 3 is otherwise covered by testing the `pub(crate)` functions it calls (`submit_ask`, `escape_cmdline`) directly, and the two edit-arm additions (`p.ask = None` in `Character`/`Space`) are covered indirectly by asserting the well-established Phase 1 convention (`p.complete = None` in those same arms) already has no direct test either — noted, not newly introduced.
