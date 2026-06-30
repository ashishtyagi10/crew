# Crew Notification System — Design

**Date:** 2026-06-29
**Status:** Implemented (tests green, clippy clean)

## Goal

Surface noteworthy pane events to the user *in-app*, so they don't have to babysit
a pane. The headline case: launch `claude` (or any long command) in a pane, tab
away, and get told when it finishes. Plus bells, output pattern matches, and pane
exits.

## Decisions (locked)

- **Events:** all four — (1) agent/command finished, (2) terminal bell,
  (3) output pattern match, (4) pane process exited.
- **Surfacing:** in-app only (no OS desktop notifications). Reuse the existing
  status-flash + LOG ring buffer (`status.rs`), which already renders on the input
  bar and in the sidebar LOG section.
- **Scope:** all panes, including the focused one. (The LOG persists regardless of
  focus, so a focused-pane event is still recorded even though its activity glyph
  clears on the next frame.)

## Non-goals (YAGNI)

- No OS/desktop notifications, no sound. (`osascript` path explicitly out.)
- No new sidebar panel or pane-border glyph in v1 — notifications ride the
  existing LOG + status flash. A dedicated NOTIFY card is a future enhancement.
- No per-pane pattern configuration; patterns are global for v1.

## Architecture

Detection already has a single hub: `poll.rs::poll_panes`, the per-tick loop that
drains every pane. All four events are detected there (or at the existing
exit-reap site), collected into a local `Vec` during the borrow of `self.panes`,
then surfaced after the loop via a single `CrewApp::notify(...)` call. This mirrors
the existing `collected_actions` pattern and avoids a borrow conflict (the pane
loop holds `&mut self.panes`; surfacing needs `&mut self`).

```
poll_panes (per tick)
  ├─ per-pane loop (borrows self.panes mut)
  │    ├─ bell:        t.pty.take_bell()        → collect (Bell, pane, "")
  │    ├─ pattern:     t.pty.take_matches()     → collect (Pattern, pane, hit)
  │    └─ (procnames)  cmd Some(x)→None & dur≥N → collect (AgentDone, pane, x)
  ├─ exit-reap site:   t.pty.exited()           → collect (Exited, pane, "")
  └─ post-loop:        for ev in collected → self.notify(ev)
                         → Notifier.record() (throttle) → set_status(msg)
```

### New module: `crates/crew-app/src/notify.rs`

Pure, unit-testable notification logic — no rendering, no PTY.

```rust
pub enum NotifyKind { AgentDone, Bell, Pattern, Exited }

pub struct Notification {
    pub kind: NotifyKind,
    pub pane: String,    // human label (title_text or folder)
    pub detail: String,  // e.g. the finished command or matched pattern
    pub at: Instant,
}

#[derive(Default)]
pub struct Notifier {
    recent: VecDeque<Notification>, // bounded (cap ~32) — throttle + /notify list
}

impl Notifier {
    /// Record an event. Returns the formatted status line to flash/log, or
    /// `None` when throttled (identical kind+pane+detail within COOLDOWN).
    pub fn record(&mut self, kind, pane, detail, now: Instant) -> Option<String>;
}
```

- **Throttle:** suppress an identical `(kind, pane, detail)` seen within
  `COOLDOWN` (~10s). Stops a chatty pattern (`error` printed in a loop) or a
  spammy bell from flooding the LOG.
- **Message formatting** (examples):
  - AgentDone → `"✓ claude finished in crew"`
  - Bell      → `"🔔 bell in build"`
  - Pattern   → `"⚑ matched \"error\" in api"`
  - Exited    → `"⊗ shell exited in crew"`
  (Glyph choice TBD-free: pick plain ASCII-safe markers consistent with existing
  LOG style; final glyphs decided in implementation, kept simple.)

### CrewApp glue (`app.rs` + a small `impl` near `status.rs`)

- Add field `notifier: Notifier` to `CrewApp` (Default-constructible, so
  `CrewApp::default()` in tests still works).
- Add method:
  ```rust
  fn notify(&mut self, kind: NotifyKind, pane: String, detail: String) {
      if !self.config.notify { return; }
      // per-kind enable gate from config
      if let Some(msg) = self.notifier.record(kind, pane, detail, Instant::now()) {
          self.set_status(msg); // existing: flash + timestamped LOG entry
      }
  }
  ```

### Detection details (`poll.rs`)

1. **Bell** — `rang` is already computed at the top of the pane loop. When
   `rang && config.notify_bell`, collect `(Bell, pane_name, "")`.
2. **Pattern** — after `try_read`, drain `t.pty.take_matches()` (new, below). For
   each hit, collect `(Pattern, pane_name, hit)`.
3. **AgentDone** — in the `procnames.due()` block where `t.cmd` is reassigned:
   - On `None → Some` transition: set `t.cmd_since = Some(now)`.
   - On `Some(old) → None` transition: if `t.cmd_since`'s elapsed `>=
     config.notify_min_secs`, collect `(AgentDone, pane_name, old)`; reset
     `cmd_since = None`.
   - `Some(a) → Some(b)` (one command launches another): no event; leave
     `cmd_since` as the original start.
   - Extract the decision into a pure helper for testing:
     `fn agent_done(old: Option<&str>, new: Option<&str>, since: Option<Instant>,
     min: Duration, now: Instant) -> AgentDoneOutcome` returning
     `{ event: Option<String>, new_since: Option<Instant> }`.
4. **Exited** — at the existing reap site (`poll.rs:124-136`), before
   `close_pane(i)`, if `config.notify_exit`, collect `(Exited, pane_name, "")`
   using the pane's `title_text()`.

`pane_name` = `p.title_text()` captured during the loop (human-friendly).

### State additions

- `TermPane` (`pane.rs`): add `cmd_since: Option<Instant>` — when the current
  foreground command started. Defaulted to `None` in `spawn_pane`.

### PTY pattern scanning (`crates/crew-term/src/pty.rs` + a helper)

Patterns are matched on the **raw output stream** (sees fast-scrolled output too),
bounded and ANSI-aware. Mirrors the existing `take_bell`/`take_cwd` accessor style.

- `PtyTerm` gains:
  - `watch: Vec<String>` (lowercased substrings) + `set_watch_patterns(&mut self, Vec<String>)`.
  - `hits: Vec<String>` + `take_matches(&mut self) -> Vec<String>` (drains).
  - A bounded carry `tail: String` (≤256 chars) so a pattern split across two
    8 KiB chunks still matches.
- Scanning runs inside the existing read path (`try_read`, right after
  `core.feed(&chunk)`), bounded by the same `READ_BUDGET`.
- **Pure helper** (testable in isolation):
  `fn scan(tail: &mut String, chunk: &[u8], patterns: &[String]) -> Vec<String>`
  — lossily decodes, strips ANSI CSI/OSC escape sequences via a tiny state filter,
  appends to `tail`, case-insensitively searches each pattern, trims `tail` to its
  cap. Returns the patterns that matched this call.
- **Known limitation (documented):** matching is on stripped text; exotic escape
  interleaving mid-word could miss a match. Acceptable for v1 (plain markers like
  `error`, `Build succeeded` work).
- The app sets patterns on every terminal pane's PTY at spawn and whenever config
  changes, via a `CrewApp::apply_notify_patterns()` helper that iterates panes.

### Config (`config.rs`)

New `#[serde(default)]` fields (all keep existing TOML backward-compatible):

| field               | type        | default | meaning                                  |
|---------------------|-------------|---------|------------------------------------------|
| `notify`            | bool        | `true`  | master on/off                            |
| `notify_agent_done` | bool        | `true`  | command-finished events                  |
| `notify_bell`       | bool        | `true`  | bell events                              |
| `notify_exit`       | bool        | `true`  | pane-exit events                         |
| `notify_min_secs`   | u64         | `10`    | min foreground-command duration to notify|
| `notify_patterns`   | Vec<String> | `[]`    | global watch substrings                  |

- `clamped()`: clamp `notify_min_secs` to `1..=3600`. `notify_patterns` filtered to
  non-empty entries.
- Update `Default`, `clamped`, and the `round_trip`/`clamped_out_of_range` tests to
  include the new fields.

### Slash command `/notify` (`dispatch.rs` + `suggest::COMMANDS`)

- `/notify` — flash current settings + recent-notification count.
- `/notify on` | `/notify off` — toggle `config.notify`, save, reflect live.
- `/notify add <substr>` — append to `notify_patterns`, save,
  `apply_notify_patterns()`.
- `/notify clear` — empty `notify_patterns`, save, re-apply.

Wire as a prefix match in the `other =>` arm (like `/find `, `/name `), and add
`"notify"` to `suggest::COMMANDS`.

## Data flow summary

PTY output → `try_read` feeds grid + `scan()` collects pattern hits;
foreground PID → `procname` → `t.cmd` transitions; bell/exit flags →
all collected in `poll_panes` → `CrewApp::notify` → `Notifier.record` (throttle) →
`set_status` → input-bar flash + LOG ring → sidebar LOG render.

## Error handling

- All detection is best-effort and non-fatal: a missing PID, an undecodable byte
  chunk, or an empty pattern list simply yields no event.
- Everything runs on the winit main thread, so all added work is **bounded**:
  pattern scan is capped by `READ_BUDGET` + a ≤256-char tail; `Notifier.recent` is
  a fixed-cap deque. No new threads, no blocking I/O (honors the main-thread
  memory note).
- Config save is a no-op under `cfg!(test)` (existing behavior) — tests don't
  touch the real config.

## Testing strategy (TDD)

- `notify.rs`: throttle suppresses identical events within cooldown; distinct
  events pass; deque cap enforced; message formatting per kind.
- `pty.rs` `scan()` helper: matches across a chunk boundary via `tail`;
  case-insensitive; ignores ANSI CSI/OSC; multiple patterns; empty patterns →
  no-op; `tail` stays bounded.
- `agent_done()` helper: `Some→None` past threshold fires; under threshold
  suppressed; `None→Some` sets start; `Some→Some` no event.
- `config.rs`: new defaults; `notify_min_secs` clamp; round-trip with patterns.
- `dispatch.rs`: `/notify add`/`on`/`off`/`clear` mutate config as expected.

## Files touched

- New: `crates/crew-app/src/notify.rs`
- `crates/crew-app/src/poll.rs` — detection + post-loop surfacing
- `crates/crew-app/src/pane.rs` — `TermPane.cmd_since`
- `crates/crew-app/src/app.rs` — `notifier` field, module decl
- `crates/crew-app/src/status.rs` (or app.rs) — `notify()`, `apply_notify_patterns()`
- `crates/crew-app/src/config.rs` — new fields + clamp + tests
- `crates/crew-app/src/dispatch.rs` + `suggest.rs` — `/notify`
- `crates/crew-app/src/main.rs` — `mod notify;`
- `crates/crew-term/src/pty.rs` — watch patterns, `take_matches`, `scan()` helper
- `crates/crew-app/src/spawn.rs` — set patterns on newly spawned panes
