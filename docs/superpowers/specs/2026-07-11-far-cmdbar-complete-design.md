# Far Command Bar: Smart Completion (Phase 1) — Design

**Date:** 2026-07-11
**Status:** Approved
**Scope:** app-only (crew-app/farpane). Phase 2 (AI suggestions) is a separate
spec: `2026-07-11-far-cmdbar-ai-design.md`.

## Goal

The Far pane's command bar completes like a good shell and remembers like
fish: Tab completes commands and paths, history persists across sessions,
and the best history match shows as accept-with-Right ghost text.

## Constraints

- Never block the winit thread unboundedly: a completion reads at most ONE
  directory (the one the token resolves into); the $PATH binary list is
  scanned once per session on a background thread and cached (empty until
  ready — completion degrades gracefully, no waiting).
- Tab stays contextual, matching the existing `typing` flag in
  `farpane/keys.rs`: bar empty → Tab switches panels (unchanged); bar
  non-empty → Tab completes/cycles.
- The engine is pure: all functions take (text, cwd, binaries) as
  parameters and return data — unit-testable against tempdirs, no globals.
- History file lives beside the existing chat-input history (same dirs
  base), named `far-history`, newline-delimited, deduped-adjacent, capped
  at 500 entries (oldest dropped), loaded once per pane.
- Ghost text renders in `text_muted` after the caret and is never part of
  `cmdline` until accepted; all rendered text follows the existing width
  helpers.

## 1. Completion engine — `farpane/complete.rs` (new)

```rust
pub(crate) enum TokenKind { Command, Path }

/// Which token the caret sits in (caret is always end-of-line today) and
/// the token's text: first whitespace-separated word → Command, later
/// words → Path. `cd`'s argument is always Path.
pub(crate) fn caret_token(text: &str) -> (TokenKind, &str)

/// Ranked candidates for the token: Command → builtins ("cd") + PATH
/// binaries prefix-matched; Path → entries of the token's parent dir
/// (expanded via pathexpand against `cwd`) prefix-matched, dirs suffixed
/// with '/'. Case-sensitive first, then case-insensitive. Bounded by one
/// read_dir.
pub(crate) fn candidates(text: &str, cwd: &Path, binaries: &[String]) -> Vec<String>

/// The new full cmdline after applying candidate `i` to the caret token.
pub(crate) fn apply(text: &str, candidate: &str) -> String
```

Completion state on `FarPane`: `complete: Option<CycleState { candidates, i, prefix }>`
— invalidated by any edit; Tab with state advances `i` (wraps), without
state builds it. A single candidate applies immediately (plus trailing
space for commands, none for dirs so deeper completion chains).

## 2. $PATH binary cache

`FarPane` gains `bins: Arc<OnceLock<Vec<String>>>`; first Tab that needs
Command candidates spawns a thread that reads each `$PATH` dir once
(file_name + executable bit), sorts, dedupes, and sets the lock. Until
set, Command completion falls back to builtins only.

## 3. History + ghost text — `farpane/cmdhist.rs` (new)

```rust
pub(crate) struct CmdHistory { entries: Vec<String>, cursor: Option<usize> }
impl CmdHistory {
    pub(crate) fn load() -> Self          // dirs-based file, missing → empty
    pub(crate) fn push(&mut self, cmd: &str)   // skip empty/adjacent-dupe, cap 500, save
    pub(crate) fn prev/next(&mut self, current: &str) -> Option<&str>  // Up/Down
    pub(crate) fn ghost(&self, prefix: &str) -> Option<&str>  // newest entry starting with prefix (prefix non-empty)
}
```

- `run_cmdline` pushes every executed command (including `cd …`).
- Up/Down cycle history into the bar only while the bar has focus-typing
  semantics (same guard as other cmdline keys); Down past newest restores
  what was being typed.
- Ghost: while typing, the newest history entry strictly extending the
  current text renders dim after the caret. Right (or End) with a ghost
  visible accepts it into `cmdline`. Enter runs only the real `cmdline`
  (never the ghost).

## 4. Rendering

`farpane/render.rs`: the cmdline row draws `cmdline` as today, then the
ghost remainder in `text_muted`. Tab-cycling shows the current candidate
applied in-line (state lives in `cmdline` directly — cycling REPLACES the
token, Esc restores the original prefix captured in `CycleState`).

## 5. Testing

- Engine: tempdir fixtures — command vs path token split, `cd` arg is
  Path, prefix ranking, dir slash suffix, one-candidate immediate apply,
  apply() token replacement mid-command.
- History: tempdir HOME — push/dedupe/cap, prev/next cycling with
  restore-typed-text, ghost prefix rules (no ghost on empty bar).
- Keys: Tab contextual behavior (empty → panel switch preserved, typing →
  complete), Esc during cycle restores, Right accepts ghost.
- Render: ghost cells use text_muted and never enter cmdline.

## Out of scope (YAGNI)

Mid-line caret editing (bar is append/pop today; completion assumes caret
at end), fuzzy matching, per-directory history, shell alias/function
completion, the AI mode (Phase 2).
