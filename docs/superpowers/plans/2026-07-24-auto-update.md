# Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Crew checks for and installs updates automatically (quietly), keeps `/update` loud and `/restart` manual, parks a blinking nav-legend reminder until the user restarts, and restart provably loads the newest installed binary.

**Architecture:** A tiny scheduler (`autoupdate.rs`) fires the EXISTING update worker silently from the poll tick; `UpdateState` gains a `silent` flag that suppresses the UPDATE card and status noise; `Installed` (either mode) parks `CrewApp.parked_update`, which `navcard` renders as a blinking → steady accent legend; `detach.rs` factors its command construction into a testable `restart_command()`.

**Tech Stack:** Rust; existing worker/poll/anim/card infrastructure; `cargo test -p crew-app`.

**Spec:** `docs/superpowers/specs/2026-07-24-auto-update-design.md`

## Global Constraints

- Workspace root `/Users/atyagi/code/crew`; all work in `crates/crew-app`.
- NOTHING new on the winit thread: network/disk stays in the existing worker (`updatefetch::spawn_worker`); the scheduler only compares `Instant`s.
- No auto-restart, no restart signal — the invariant test `install_parks_then_clears_without_restarting` must keep passing.
- Launch delay 30 s; check cadence 6 h (constants `FIRST_CHECK` / `CHECK_EVERY` in `autoupdate.rs`).
- Reminder blink: `attention.rs`'s constants (`BLINK_MS` 400 half-period, `PULSE_MS` 4000 pulse window, then steady accent) on the shared `anim::now_ms()` clock; redraws driven only inside the pulse window.
- Reminder legend text format, exactly: `crew v<current> → v<new> · /restart` (current = `env!("CARGO_PKG_VERSION")`).
- `/update`'s visible behavior is unchanged (loud card, status messages, 5 s linger).
- Pre-commit runs `cargo fmt --check` + `cargo check`; run `cargo fmt` before every commit.
- Tests: `cargo test -p crew-app <module>` per task; full `cargo test -p crew-app` before each commit.

---

### Task 1: Silent scheduler + parked state

**Files:**
- Create: `crates/crew-app/src/autoupdate.rs`
- Modify: `crates/crew-app/src/update.rs` (silent flag, silent terminal handling, parked capture)
- Modify: `crates/crew-app/src/app.rs` (two new fields), `crates/crew-app/src/main.rs` (mod decl), `crates/crew-app/src/poll.rs` (~line 260 wiring), `crates/crew-app/src/navcard.rs` (~line 33: gate the UPDATE card on `!silent`)
- Test: `autoupdate.rs` inline tests + `update.rs` tests module

**Interfaces:**
- Produces: `pub(crate) struct AutoUpdate { next_check: Instant }` with `AutoUpdate::new(now) -> Self` (first due = now + `FIRST_CHECK` = 30 s) and `fn take_due(&mut self, now: Instant) -> bool` (true at/after the deadline, and re-arms to now + `CHECK_EVERY` = 6 h). `CrewApp.autoupdate: AutoUpdate`, `CrewApp.parked_update: Option<String>` (version string, set on ANY `Installed`). `UpdateState.silent: bool`; `CrewApp::start_auto_update()` (silent, no status); a manual `/update` while a silent run is in flight flips `silent = false` ("upgrade to loud") instead of refusing. Task 2 consumes `parked_update`.
- Consumes: `updatefetch::spawn_worker` (unchanged), `poll.rs`'s existing `poll_update` call site.

- [ ] **Step 1: Write the failing tests**

In `autoupdate.rs` (bottom, `#[cfg(test)] mod tests`):

```rust
#[test]
fn first_check_waits_the_launch_delay_then_rearms_six_hourly() {
    let t0 = Instant::now();
    let mut a = AutoUpdate::new(t0);
    assert!(!a.take_due(t0), "not due immediately at launch");
    assert!(!a.take_due(t0 + FIRST_CHECK - Duration::from_secs(1)));
    assert!(a.take_due(t0 + FIRST_CHECK), "due after the launch delay");
    assert!(!a.take_due(t0 + FIRST_CHECK), "take_due re-arms — not due twice");
    assert!(!a.take_due(t0 + FIRST_CHECK + CHECK_EVERY - Duration::from_secs(1)));
    assert!(a.take_due(t0 + FIRST_CHECK + CHECK_EVERY));
}
```

In `update.rs` tests (reuse the existing channel-injection idiom from `install_parks_then_clears_without_restarting`):

```rust
#[test]
#[allow(clippy::field_reassign_with_default)]
fn installed_parks_the_update_version_in_both_modes() {
    for silent in [true, false] {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = CrewApp::default();
        app.update = Some(UpdateState::new_with(rx, silent));
        tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
        app.poll_update(Instant::now());
        assert_eq!(app.parked_update.as_deref(), Some("9.9.9"), "silent={silent}");
    }
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn silent_terminal_notes_clear_without_lingering() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new_with(rx, true));
    tx.send(UpdateMsg::UpToDate("1.0.0".into())).unwrap();
    let now = Instant::now();
    app.poll_update(now);
    // Silent up-to-date does NOT park a 5s note card — cleared by the next tick.
    app.poll_update(now);
    assert!(app.update.is_none(), "silent terminal state must not linger");
    assert!(app.parked_update.is_none());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn manual_update_upgrades_a_silent_run_to_loud() {
    let (_tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new_with(rx, true));
    app.start_update();
    let u = app.update.as_ref().unwrap();
    assert!(!u.silent, "manual /update takes over the silent run loudly");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app autoupdate; cargo test -p crew-app update::tests`
Expected: compile FAIL (`AutoUpdate`, `new_with`, `silent`, `parked_update` undefined).

- [ ] **Step 3: Implement**

`autoupdate.rs`:

```rust
//! Auto-update scheduler: decides WHEN the silent background check runs.
//! Pure Instant arithmetic — the network/disk work stays on the update
//! worker thread (`updatefetch`), so the winit thread never blocks.
use std::time::{Duration, Instant};

/// Launch settles first; the first quiet check runs shortly after startup.
pub(crate) const FIRST_CHECK: Duration = Duration::from_secs(30);
/// Steady-state cadence between quiet checks.
pub(crate) const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// The next-check deadline. `take_due` answers "fire now?" and re-arms.
pub(crate) struct AutoUpdate {
    next_check: Instant,
}

impl AutoUpdate {
    pub(crate) fn new(now: Instant) -> Self {
        Self { next_check: now + FIRST_CHECK }
    }
    /// True once per elapsed deadline; re-arms to the 6 h cadence.
    pub(crate) fn take_due(&mut self, now: Instant) -> bool {
        if now < self.next_check {
            return false;
        }
        self.next_check = now + CHECK_EVERY;
        true
    }
}
```

`update.rs`:
- `UpdateState` gains `pub(crate) silent: bool`; rename the private ctor to `new_with(rx, silent)` (keep `new(rx)` delegating with `silent: false` if other call sites want it, or update the one call site — implementer's choice, state it).
- In `apply()`, silent mode's terminal arms clear immediately instead of lingering: for `UpToDate`/`Failed` (and `Installed`'s card — the reminder carries the news), set `self.deadline = Some(now)` when `self.silent` (loud mode keeps `NOTE_TTL`).
- `poll_update`: after `drain`, if the stage is `Stage::Done(v)` and `self.parked_update.is_none()`, set `self.parked_update = Some(v.clone())` — BOTH modes; this is the one place `Installed` is captured.
- `start_update` (manual): if a run exists and is silent → set its `silent = false`, post the "checking for updates…" status, return (upgrade-to-loud). If a LOUD run is animating → keep today's "update already in progress" refusal. Otherwise spawn as today.
- New `start_auto_update`: no-op if `self.update.is_some()` OR `self.parked_update.is_some()` (an installed version is already parked — nothing to gain until restart); else `self.update = Some(UpdateState::new_with(spawn_worker(), true))` with NO status message and NO redraw call.

`app.rs`: add `pub(crate) autoupdate: crate::autoupdate::AutoUpdate` (init `AutoUpdate::new(Instant::now())` — mirror however `CrewApp::default`/new initializes time-based fields) and `pub(crate) parked_update: Option<String>` (init `None`).

`main.rs`: add `mod autoupdate;` alongside the existing `mod update;` cluster (~line 149).

`poll.rs` (~line 260, just above the existing update-drive block):

```rust
        // Quiet auto-update: fire the same worker the manual /update uses,
        // silently, shortly after launch and then six-hourly. Restart stays
        // manual — an install only parks the nav-legend reminder.
        if self.autoupdate.take_due(Instant::now()) {
            self.start_auto_update();
        }
```

and change the drive condition `if self.update.is_some()` to run `poll_update` the same way (unchanged otherwise).

`navcard.rs` (~line 33): gate the UPDATE card block on loud runs only — `if let Some(u) = self.update.as_ref().filter(|u| !u.silent)`. Also check `chrome::stats_card_rect(…, self.update.is_some())` on the neighboring line: pass `self.update.as_ref().is_some_and(|u| !u.silent)` so the stats card doesn't reserve space for an invisible card.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app autoupdate && cargo test -p crew-app update && cargo test -p crew-app` (full crate — navcard/chrome tests must keep passing).
Expected: ALL PASS, including `install_parks_then_clears_without_restarting` untouched.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/autoupdate.rs crates/crew-app/src/update.rs crates/crew-app/src/app.rs crates/crew-app/src/main.rs crates/crew-app/src/poll.rs crates/crew-app/src/navcard.rs
git commit -m "feat(update): silent six-hourly auto-update parks installs for /restart"
```

---

### Task 2: Blinking restart reminder in the nav legend

**Files:**
- Create: `crates/crew-app/src/restartnote.rs` (pure legend text/color/redraw helpers)
- Modify: `crates/crew-app/src/navcard.rs` (~line 41 legend), `crates/crew-app/src/panelcard.rs` (legend-color variant), `crates/crew-app/src/poll.rs` (drive redraws during the pulse window), `crates/crew-app/src/main.rs` (mod decl)
- Test: `restartnote.rs` inline tests

**Interfaces:**
- Consumes: `CrewApp.parked_update: Option<String>` (Task 1). `attention.rs` constants `BLINK_MS`/`PULSE_MS`; `anim::now_ms()`.
- Produces: `restartnote::legend(new_version: &str) -> String`; `restartnote::legend_fg(now_ms: u64, parked_at_ms: u64) -> (u8, u8, u8)` (blink accent↔legend_off inside the pulse window, steady accent after); `restartnote::animating(now_ms, parked_at_ms) -> bool`; `panelcard::push_card_titled(scenes, rect, cw, ch, legend, title_fg, content)` (existing `push_card` delegates to it with `legend_off`). `CrewApp.parked_update` becomes `Option<(String, u64)>` — version + parked-at on the anim clock (Task 1's plain `Option<String>` is upgraded here; update Task 1's two tests to match with `.0`).

- [ ] **Step 1: Write the failing tests**

In `restartnote.rs`:

```rust
#[test]
fn legend_names_both_versions_and_the_restart_command() {
    let s = legend("9.9.9");
    assert!(s.starts_with(concat!("crew v", env!("CARGO_PKG_VERSION"))), "{s}");
    assert!(s.contains("\u{2192} v9.9.9"), "{s}"); // →
    assert!(s.ends_with("\u{b7} /restart"), "{s}"); // ·
}

#[test]
fn legend_blinks_through_the_pulse_window_then_holds_accent() {
    let accent = crate::palette::accent();
    let t0 = 10_000u64;
    // Inside the pulse window the fg alternates on the BLINK_MS half-period.
    let a = legend_fg(t0, t0);
    let b = legend_fg(t0 + crate::attention::BLINK_MS, t0);
    assert_ne!(a, b, "must alternate each half-period");
    assert!(a == accent || b == accent, "one phase is the accent");
    // After the pulse window: steady accent, and no more redraw driving.
    let late = t0 + crate::attention::PULSE_MS + 1;
    assert_eq!(legend_fg(late, t0), accent);
    assert!(animating(t0 + 1, t0));
    assert!(!animating(late, t0));
}
```

(Use `crate::palette::test_guard()` first if `palette::accent()` needs it — copy the idiom from `suggested_command_highlights_the_bar_and_shows_the_accept_hint` in `render_tests.rs`.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app restartnote`
Expected: compile FAIL.

- [ ] **Step 3: Implement**

`restartnote.rs`:

```rust
//! The parked-update restart reminder: legend text and blink styling for
//! the nav stats card while a newer installed binary waits for /restart.
//! Pure helpers — state lives on CrewApp.parked_update, painting in navcard.

/// `crew v<current> → v<new> · /restart`
pub(crate) fn legend(new_version: &str) -> String {
    format!(
        concat!("crew v", env!("CARGO_PKG_VERSION"), " \u{2192} v{} \u{b7} /restart"),
        new_version
    )
}

/// Accent↔dim alternation on the attention clock for the first PULSE_MS,
/// then steady accent — same cost model as pane attention markers.
pub(crate) fn legend_fg(now_ms: u64, parked_at_ms: u64) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    if animating(now_ms, parked_at_ms)
        && ((now_ms - parked_at_ms) / crate::attention::BLINK_MS) % 2 == 1
    {
        return t.legend_off;
    }
    crate::palette::accent()
}

/// True only inside the blink window — the only time redraws are driven.
pub(crate) fn animating(now_ms: u64, parked_at_ms: u64) -> bool {
    now_ms.saturating_sub(parked_at_ms) < crate::attention::PULSE_MS
}
```

(Adjust to `palette::accent()`'s real return type/name — check `palette.rs`; if the accessor is `accent_color()` returning a render `Color`, add/use whichever variant returns the `(u8,u8,u8)` tuple the card API takes — `titled_card` takes tuples.)

`panelcard.rs`: add `push_card_titled(…, title_fg: (u8, u8, u8), …)` — body is today's `push_card` with `title_fg` in place of `theme().legend_off`; `push_card` becomes a one-line delegate.

Task-1 field upgrade: `parked_update: Option<(String, u64)>`; the capture site in `poll_update` stamps `(v.clone(), crate::anim::now_ms())`; Task 1's tests compare `.0`; `start_auto_update`'s parked guard uses `.is_some()` unchanged.

`navcard.rs` (~line 41):

```rust
        let (legend, legend_fg) = match &self.parked_update {
            Some((v, at)) => (
                crate::restartnote::legend(v),
                crate::restartnote::legend_fg(crate::anim::now_ms(), *at),
            ),
            None => (
                concat!("crew v", env!("CARGO_PKG_VERSION")).to_string(),
                crew_theme::theme().legend_off,
            ),
        };
        crate::panelcard::push_card_titled(scenes, sb, cw, ch, &legend, legend_fg, |cols, rows| {
            sidebar.cells(cols, rows, &pane_rows, log)
        });
```

`poll.rs`: in the poll tick (near the update-drive block), drive redraws while the reminder is blinking:

```rust
        if let Some((_, at)) = &self.parked_update {
            if crate::restartnote::animating(crate::anim::now_ms(), *at) {
                any_changed = true;
            }
        }
```

`main.rs`: `mod restartnote;`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app restartnote && cargo test -p crew-app update && cargo test -p crew-app` (full crate).
Expected: ALL PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/restartnote.rs crates/crew-app/src/navcard.rs crates/crew-app/src/panelcard.rs crates/crew-app/src/poll.rs crates/crew-app/src/update.rs crates/crew-app/src/app.rs crates/crew-app/src/main.rs
git commit -m "feat(update): nav legend blinks a restart reminder for parked installs"
```

---

### Task 3: Pin restart-loads-latest + comment truth

**Files:**
- Modify: `crates/crew-app/src/detach.rs` (factor `restart_command`), `crates/crew-app/src/dispatch.rs` (~line 23 stale comment)
- Test: `detach.rs` tests module

**Interfaces:**
- Produces: `pub fn restart_command() -> anyhow::Result<(std::path::PathBuf, Vec<String>)>` — the exact (exe, args) `spawn_detached_copy` will spawn; `spawn_detached_copy` consumes it.

- [ ] **Step 1: Write the failing test**

In `detach.rs` tests:

```rust
#[test]
fn restart_reexecs_the_installed_binary_path() {
    let (exe, args) = restart_command().unwrap();
    // self_update atomically replaces the file at current_exe()'s path, so
    // re-execing that path is what makes /restart load the newest install.
    assert_eq!(exe, std::env::current_exe().unwrap());
    assert!(
        !args.iter().any(|a| a == "--detached" || a.starts_with("--detach")),
        "detach flags must be stripped: {args:?}"
    );
}
```

(Match the stripped flags to whatever `strip_detach_flags` actually removes — read it and assert its real markers rather than the guesses above.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p crew-app detach`
Expected: compile FAIL (`restart_command` undefined).

- [ ] **Step 3: Implement**

In `detach.rs`, factor lines 44–45 of `spawn_detached_copy`:

```rust
/// The (exe, args) a restart will spawn: OUR OWN PATH with detach flags
/// stripped. self_update replaces the file at this path atomically, so
/// re-exec here is the guarantee that /restart (and any relaunch) always
/// loads the newest installed binary.
pub fn restart_command() -> anyhow::Result<(std::path::PathBuf, Vec<String>)> {
    let exe = std::env::current_exe()?;
    let args = strip_detach_flags(std::env::args().skip(1));
    Ok((exe, args))
}
```

`spawn_detached_copy` calls it and keeps the rest unchanged. In `dispatch.rs` (~line 23), fix the stale comment: "Crew auto-restarts into the new build — no separate shell" → "the new binary applies on /restart — Crew never restarts itself".

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app detach && cargo test -p crew-app` (full crate).
Expected: ALL PASS.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/detach.rs crates/crew-app/src/dispatch.rs
git commit -m "test(restart): pin that /restart re-execs the installed binary path"
```
