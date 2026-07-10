# Roster Animation & Live Feedback (Phase A) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The crew pane's agent roster animates: bars ease with sub-cell fill, the working agent breathes toward the accent, handoffs flash, and token counts roll — with zero idle repaints.

**Architecture:** A new `chatanim.rs` owns time-parameterized eased values (`Eased`) and the per-agent animation store (`RosterAnim`) on `ChatPane`. Targets are set where data lands (`absorb_stats` / `absorb_activity`, which have `&mut self`); renders read pure `value(now)` (no interior mutability). `chatchips.rs` renders eased fractions with left-eighth partial blocks and blends colors via the existing `anim::tri`/`anim::lerp_rgb`. Redraw tails ride `paneview::pane_animating` so the "idle never repaints" rule holds.

**Tech Stack:** Rust (workspace v0.5.57), existing crates only — no new dependencies. Key existing helpers: `anim::now_ms()`, `anim::tri(now, period_ms) -> f32`, `anim::lerp_rgb(a, b, t)`, `gauges::fill_color(frac)`, `crate::palette::accent()`.

**Spec:** `docs/superpowers/specs/2026-07-09-roster-animation-design.md`

## Global Constraints

- All render functions take `now: u64` as a parameter; only the frame loop reads `anim::now_ms()`. Tests inject `now`.
- Never animate while idle: `ChatPane::anim_active(now)` must return `false` once everything settles, and `paneview::pane_animating` is the only redraw hook.
- Ease windows: bars 250ms, tok 400ms, flash 400ms; pulse period 1600ms; pulse and flash blend amplitudes ≤ 0.25 and ≤ 0.35 respectively.
- Percent text always shows the TARGET value; only the bar sweeps.
- Every glyph rendered stays width-1 (left-eighth blocks `▉▊▋▌▍▎▏` U+2589..U+258F, `█` U+2588, `░` U+2591).
- Targets are set only in `&mut self` absorb paths — reads are pure (`Eased::value(now)` takes `&self`).
- Pre-commit hook runs `cargo fmt` + `cargo check`: run `cargo fmt` before every commit; introduce no new warnings.
- Existing chatchips alignment tests must keep passing unmodified in their assertions (signatures may gain a `now` argument).

---

### Task 1: `chatanim.rs` — Eased values and the RosterAnim store

**Files:**
- Create: `crates/crew-app/src/chatanim.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod chatanim;` alphabetically, right after `mod chat;` and before `mod chatbody;`)

**Interfaces:**
- Produces (consumed by Tasks 2–5):
  - `pub(crate) struct Eased` with `pub(crate) fn set_target(&mut self, now: u64, v: f32)`, `pub(crate) fn value(&self, now: u64) -> f32`, `pub(crate) fn settled(&self, now: u64) -> bool`, constructor `Eased::new(window_ms: u64)`
  - `pub(crate) struct RosterAnim` with fields/methods below; `RosterAnim::new()`
  - `pub(crate) const BAR_MS: u64 = 250;` `pub(crate) const TOK_MS: u64 = 400;` `pub(crate) const FLASH_MS: u64 = 400;`
  - `RosterAnim::set_ctx(&mut self, agent: &str, now: u64, frac: f32)`, `set_shr`, `set_tok(&mut self, agent: &str, now: u64, tokens: f32)`
  - `RosterAnim::ctx(&self, agent: &str, now: u64) -> f32`, `shr`, `tok(&self, agent: &str, now: u64) -> f32` (0.0 for unknown agents)
  - `RosterAnim::ctx_target(&self, agent: &str) -> f32`, `shr_target` (0.0 for unknown)
  - `RosterAnim::flash(&mut self, agent: &str, now: u64)` and `RosterAnim::flash_t(&self, agent: &str, now: u64) -> f32` (1.0 fresh → 0.0 expired, linear)
  - `RosterAnim::active(&self, now: u64) -> bool`
  - `RosterAnim::gc(&mut self, now: u64)` — drops expired flash entries (called from `flash()`)

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-app/src/chatanim.rs` containing ONLY the module doc and tests (implementation comes in Step 3):

```rust
//! Roster animation state: eased values (bars, token count-up) and one-shot
//! handoff flashes for the crew pane's agent rows. Time is always a `now`
//! parameter (tests inject it); reads are pure so render paths take `&self`,
//! and targets are set only from the `&mut self` absorb paths.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eased_converges_and_settles() {
        let mut e = Eased::new(250);
        e.set_target(1000, 1.0);
        assert_eq!(e.value(1000), 0.0, "starts at prior shown value");
        let mid = e.value(1125);
        assert!(mid > 0.0 && mid < 1.0, "mid-ease is between: {mid}");
        // Ease-out: the first half covers more than half the distance.
        assert!(mid > 0.5, "ease-out front-loads: {mid}");
        assert_eq!(e.value(1250), 1.0, "lands exactly on target");
        assert_eq!(e.value(9999), 1.0, "stays on target after the window");
        assert!(!e.settled(1100));
        assert!(e.settled(1250));
    }

    #[test]
    fn eased_retarget_mid_ease_starts_from_shown() {
        let mut e = Eased::new(250);
        e.set_target(0, 1.0);
        let shown = e.value(125);
        e.set_target(125, 0.0);
        // Immediately after retarget the value is what was on screen.
        assert!((e.value(125) - shown).abs() < 1e-6);
        assert_eq!(e.value(375), 0.0);
    }

    #[test]
    fn eased_zero_window_snaps() {
        let mut e = Eased::new(0);
        e.set_target(10, 0.7);
        assert_eq!(e.value(10), 0.7);
        assert!(e.settled(10));
    }

    #[test]
    fn roster_anim_defaults_and_targets() {
        let mut ra = RosterAnim::new();
        assert_eq!(ra.ctx("planner", 0), 0.0, "unknown agent reads 0");
        ra.set_ctx("planner", 1000, 0.21);
        assert_eq!(ra.ctx_target("planner"), 0.21);
        assert_eq!(ra.ctx("planner", 1250), 0.21, "settled after window");
        ra.set_tok("planner", 1000, 7200.0);
        assert_eq!(ra.tok("planner", 1400), 7200.0);
    }

    #[test]
    fn flash_fades_linearly_and_expires() {
        let mut ra = RosterAnim::new();
        ra.flash("coder", 1000);
        assert!((ra.flash_t("coder", 1000) - 1.0).abs() < 1e-6);
        assert!((ra.flash_t("coder", 1200) - 0.5).abs() < 1e-6);
        assert_eq!(ra.flash_t("coder", 1400), 0.0);
        assert_eq!(ra.flash_t("nobody", 1000), 0.0);
    }

    #[test]
    fn active_goes_false_after_everything_settles() {
        let mut ra = RosterAnim::new();
        assert!(!ra.active(0), "fresh store is inactive");
        ra.set_ctx("planner", 1000, 0.5);
        ra.flash("coder", 1000);
        assert!(ra.active(1100), "ease + flash in flight");
        assert!(ra.active(1399), "flash window (400ms) still open");
        assert!(!ra.active(1401), "everything settled → inactive");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatanim`
Expected: FAIL to compile — `Eased`, `RosterAnim` not found. (Also add `mod chatanim;` to `main.rs` now or this won't even attempt to compile the module.)

- [ ] **Step 3: Implement**

Add above the tests module in `chatanim.rs`:

```rust
use std::collections::HashMap;

/// Ease windows (ms): bar sweeps, token count-up, handoff flash.
pub(crate) const BAR_MS: u64 = 250;
pub(crate) const TOK_MS: u64 = 400;
pub(crate) const FLASH_MS: u64 = 400;

/// Cubic ease-out on `t` in 0..=1: fast start, gentle landing.
fn ease_out(t: f32) -> f32 {
    let inv = 1.0 - t.clamp(0.0, 1.0);
    1.0 - inv * inv * inv
}

/// One value easing toward a target over a fixed window. Reads are pure —
/// `value(now)` interpolates from what was on screen when the target was set.
pub(crate) struct Eased {
    from: f32,
    target: f32,
    since_ms: u64,
    window_ms: u64,
}

impl Eased {
    pub(crate) fn new(window_ms: u64) -> Self {
        Eased { from: 0.0, target: 0.0, since_ms: 0, window_ms }
    }

    /// Retarget, restarting the ease from the currently shown value.
    pub(crate) fn set_target(&mut self, now: u64, v: f32) {
        self.from = self.value(now);
        self.target = v;
        self.since_ms = now;
    }

    pub(crate) fn value(&self, now: u64) -> f32 {
        if self.window_ms == 0 || now >= self.since_ms + self.window_ms {
            return self.target;
        }
        let t = (now.saturating_sub(self.since_ms)) as f32 / self.window_ms as f32;
        self.from + (self.target - self.from) * ease_out(t)
    }

    pub(crate) fn settled(&self, now: u64) -> bool {
        self.from == self.target || now >= self.since_ms + self.window_ms
    }
}

/// Per-agent animation state for the roster grid: eased ctx/shr fractions,
/// eased token counts, and one-shot handoff flashes.
pub(crate) struct RosterAnim {
    ctx: HashMap<String, Eased>,
    shr: HashMap<String, Eased>,
    tok: HashMap<String, Eased>,
    flash: HashMap<String, u64>,
}

impl RosterAnim {
    pub(crate) fn new() -> Self {
        RosterAnim {
            ctx: HashMap::new(),
            shr: HashMap::new(),
            tok: HashMap::new(),
            flash: HashMap::new(),
        }
    }

    fn set(map: &mut HashMap<String, Eased>, window: u64, agent: &str, now: u64, v: f32) {
        map.entry(agent.to_string())
            .or_insert_with(|| Eased::new(window))
            .set_target(now, v);
    }

    fn get(map: &HashMap<String, Eased>, agent: &str, now: u64) -> f32 {
        map.get(agent).map(|e| e.value(now)).unwrap_or(0.0)
    }

    pub(crate) fn set_ctx(&mut self, agent: &str, now: u64, frac: f32) {
        Self::set(&mut self.ctx, BAR_MS, agent, now, frac);
    }
    pub(crate) fn set_shr(&mut self, agent: &str, now: u64, frac: f32) {
        Self::set(&mut self.shr, BAR_MS, agent, now, frac);
    }
    pub(crate) fn set_tok(&mut self, agent: &str, now: u64, tokens: f32) {
        Self::set(&mut self.tok, TOK_MS, agent, now, tokens);
    }

    pub(crate) fn ctx(&self, agent: &str, now: u64) -> f32 {
        Self::get(&self.ctx, agent, now)
    }
    pub(crate) fn shr(&self, agent: &str, now: u64) -> f32 {
        Self::get(&self.shr, agent, now)
    }
    pub(crate) fn tok(&self, agent: &str, now: u64) -> f32 {
        Self::get(&self.tok, agent, now)
    }

    pub(crate) fn ctx_target(&self, agent: &str) -> f32 {
        self.ctx.get(agent).map(|e| e.target).unwrap_or(0.0)
    }
    pub(crate) fn shr_target(&self, agent: &str) -> f32 {
        self.shr.get(agent).map(|e| e.target).unwrap_or(0.0)
    }

    /// Record a handoff flash for `agent`, dropping expired entries so the
    /// map never outgrows the roster.
    pub(crate) fn flash(&mut self, agent: &str, now: u64) {
        self.gc(now);
        self.flash.insert(agent.to_string(), now);
    }

    /// 1.0 at flash start → 0.0 at FLASH_MS, linear; 0.0 when never flashed.
    pub(crate) fn flash_t(&self, agent: &str, now: u64) -> f32 {
        self.flash
            .get(agent)
            .map(|&start| {
                let age = now.saturating_sub(start) as f32;
                (1.0 - age / FLASH_MS as f32).max(0.0)
            })
            .unwrap_or(0.0)
    }

    pub(crate) fn gc(&mut self, now: u64) {
        self.flash
            .retain(|_, &mut start| now.saturating_sub(start) < FLASH_MS);
    }

    /// Anything still moving? Drives the redraw tail after a turn ends.
    pub(crate) fn active(&self, now: u64) -> bool {
        let easing = |m: &HashMap<String, Eased>| m.values().any(|e| !e.settled(now));
        easing(&self.ctx)
            || easing(&self.shr)
            || easing(&self.tok)
            || self
                .flash
                .values()
                .any(|&start| now.saturating_sub(start) < FLASH_MS)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app chatanim`
Expected: 6 passed. (A dead-code warning for the new module is expected until Task 2 wires it; do NOT silence it — Task 2 removes it.)

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatanim.rs crates/crew-app/src/main.rs
git commit -m "feat(crew): chatanim — eased values + roster animation store"
```

---

### Task 2: Wire targets — absorb paths set eases and flashes

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (ChatPane field ~line 45, `absorb_stats` — the fn called at chat.rs:121)
- Modify: `crates/crew-app/src/chatflow.rs` (`absorb_activity` `("thinking", false)` arm, ~line 44)
- Test: `crates/crew-app/src/chat_tests.rs` (append)

**Interfaces:**
- Consumes: `RosterAnim` API from Task 1.
- Produces (used by Tasks 3–5): `ChatPane.anim: crate::chatanim::RosterAnim` (pub(crate) field); targets guaranteed set on every Stats/Activity absorb. Share targets are recomputed for ALL roster agents whenever any agent's stats land.

- [ ] **Step 1: Write the failing test**

Append to `crates/crew-app/src/chat_tests.rs` (this file already constructs `ChatPane` for absorb tests — match its existing constructor helper; if it builds panes via `ChatPane::new(...)`, reuse that):

```rust
#[test]
fn absorb_stats_retargets_roster_anim() {
    let mut c = test_pane(); // the file's existing ChatPane fixture helper
    // Two agents so share redistribution is observable.
    c.absorb_activity("planner".into(), "thinking", "user".into());
    c.absorb_stats(1200, "planner".into(), 800, 30_000);
    c.absorb_stats(400, "coder".into(), 200, 10_000);
    // Token target = the agent's live ctx (the tok column shows context fill).
    let now = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!((c.anim.tok("planner", now) - 30_000.0).abs() < 1.0);
    // Shares settle to ms proportions: planner 800/1000, coder 200/1000.
    assert!((c.anim.shr_target("planner") - 0.8).abs() < 1e-6);
    assert!((c.anim.shr_target("coder") - 0.2).abs() < 1e-6);
}

#[test]
fn thinking_activity_records_flash() {
    let mut c = test_pane();
    c.absorb_activity("coder".into(), "thinking", "planner".into());
    let now = crate::anim::now_ms();
    assert!(c.anim.flash_t("coder", now) > 0.9, "fresh handoff flash");
}
```

Adapt the fixture name to whatever `chat_tests.rs` actually uses (read the file first); keep the assertions identical. If `absorb_stats` has a different parameter order, match the real signature (`chat.rs` ~line 116: `self.absorb_stats(tokens, agent, ms, ctx)`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app absorb_stats_retargets`
Expected: FAIL to compile — `ChatPane` has no field `anim`.

- [ ] **Step 3: Implement**

In `chat.rs`: add the field next to `pulse` (~line 45):

```rust
    /// Roster animation state: eased bars/token counts + handoff flashes.
    pub(crate) anim: crate::chatanim::RosterAnim,
```

Initialize `anim: crate::chatanim::RosterAnim::new(),` wherever `ChatPane` is constructed (find with `grep -n "pulse: " crates/crew-app/src/*.rs` — every constructor that sets `pulse` also sets `anim`).

In the body of `absorb_stats` (chat.rs), after the existing ctx insert / agent_stats update, retarget:

```rust
        let now = crate::anim::now_ms();
        // tok column shows live context fill; ease it toward the new ctx.
        if ctx > 0 {
            self.anim.set_tok(&agent_name, now, ctx as f32);
        }
        // ctx% needs the model's limit — mirror agent_views' derivation.
        if let Some(a) = self.agents.iter().find(|a| a.name == agent_name) {
            if let Some(l) = crate::ctxlimit::context_limit(&a.model).filter(|&l| l > 0) {
                self.anim
                    .set_ctx(&agent_name, now, (ctx as f32 / l as f32).min(1.0));
            }
        }
        // Any stat changes every agent's share of the turn: retarget all.
        let sum_ms: u64 = self.agent_stats.values().map(|(_, ms)| *ms).sum();
        if sum_ms > 0 {
            for (name, (_, ms)) in self.agent_stats.iter() {
                let frac = (*ms as f32 / sum_ms as f32).min(1.0);
                self.anim.set_shr(name, now, frac);
            }
        }
```

(Use the actual local variable names in `absorb_stats` — `agent_name` above stands for its agent-name binding; `ctx` for the ctx tokens parameter. Borrow-check note: collect `(name, frac)` pairs into a `Vec` before the `set_shr` loop if iterating `self.agent_stats` while calling `&mut self.anim` methods conflicts — `self.anim` and `self.agent_stats` are disjoint fields so direct iteration compiles; the Vec fallback is only needed if a helper method takes `&mut self`.)

In `chatflow.rs` `absorb_activity`, inside the `("thinking", false)` arm (before the `self.active.push`), add:

```rust
                self.anim.flash(&agent, crate::anim::now_ms());
```

(`agent` is moved into `ActiveAgent { name: agent, .. }` at the end of that arm — call `flash(&agent, ..)` while it's still borrowable, i.e. before the push.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: all pass, including the two new tests; the Task 1 dead-code warning is gone.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chat.rs crates/crew-app/src/chatflow.rs crates/crew-app/src/chat_tests.rs
git commit -m "feat(crew): absorb paths retarget roster eases + handoff flash"
```

---

### Task 3: Eased bars with sub-cell fill in chatchips

**Files:**
- Modify: `crates/crew-app/src/chatchips.rs` (AgentView, Seg, push_segment, row_cells + its tests)
- Modify: `crates/crew-app/src/chatview.rs` (`agent_views` ~line 14, and both `row_cells` call sites — `status_rows`/`cells` pass-through of `now`)

**Interfaces:**
- Consumes: `ChatPane.anim` (Task 2).
- Produces (used by Task 4):
  - `AgentView` gains: `pub ctx_frac: f32`, `pub shr_frac: f32`, `pub flash_t: f32` (0.0 = none); `tok` becomes the EASED value rounded (`u64`), targets still drive `ctx_pct`/`share_pct` text.
  - `Seg` gains `frac: f32` (eased fill) alongside `pct: Option<u8>` (target text).
  - `pub(crate) fn partial_block(frac_cells: f32) -> Option<char>` — the fractional cell glyph for the remainder in 0.0..1.0 (None below 1/8).
  - `row_cells(views, cols, start_row, lay, now: u64)` — `now` threaded through (Task 4 uses it for the pulse).

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `chatchips.rs`:

```rust
    #[test]
    fn partial_block_selects_left_eighths() {
        assert_eq!(partial_block(0.0), None);
        assert_eq!(partial_block(0.05), None, "below 1/8 draws nothing");
        assert_eq!(partial_block(0.125), Some('\u{258F}'), "1/8 ▏");
        assert_eq!(partial_block(0.5), Some('\u{258C}'), "4/8 ▌");
        assert_eq!(partial_block(0.874), Some('\u{258A}'), "6/8 ▊");
        assert_eq!(partial_block(0.999), Some('\u{2589}'), "caps at 7/8 ▉");
    }

    #[test]
    fn segment_bar_uses_eased_frac_but_target_pct_text() {
        // frac mid-ease (0.35 of BAR_W=6 → 2 full cells + 0.1 partial → none),
        // while the pct text must read the target (50%).
        let mut cells = Vec::new();
        let pal = test_pal();
        let seg = Seg { pct: Some(50), frac: 0.35, label: "ctx", fill: (255, 0, 0) };
        push_segment(&mut cells, 0, 0, 40, seg, &pal);
        let row: String = row_text(&cells, 0); // existing test helper pattern
        assert!(row.contains("50%"), "text shows target: {row}");
        let full = row.chars().filter(|&c| c == '\u{2588}').count();
        assert_eq!(full, 2, "0.35 * 6 = 2.1 cells → 2 full blocks: {row}");
    }
```

If the tests module lacks `test_pal()`/`row_text()` helpers, add them (a `Pal` from two fixed colors; `row_text` sorts `cells` by col and collects chars — mirror how existing tests like `row_cells_have_name_state_pipes_and_ctx_shr_bars` inspect cells, reusing their approach verbatim).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatchips`
Expected: FAIL to compile — `partial_block` not found, `Seg` has no `frac`.

- [ ] **Step 3: Implement**

In `chatchips.rs`:

```rust
/// The left-eighth block for a fractional cell (0.0..1.0): ▏(1/8) … ▉(7/8).
/// None below 1/8 — an empty track cell reads cleaner than a sliver.
pub(crate) fn partial_block(frac_cells: f32) -> Option<char> {
    let eighths = (frac_cells.clamp(0.0, 1.0) * 8.0).floor() as u32;
    match eighths.min(7) {
        0 => None,
        // U+2589 ▉ is 7/8 … U+258F ▏ is 1/8: codepoint = 0x2590 - eighths.
        n => char::from_u32(0x2590 - n),
    }
}
```

Extend `Seg` with `frac: f32` and rewrite the bar loop in `push_segment`: `filled_cells = frac * BAR_W as f32`; draw `filled_cells.floor()` full `█` in `seg.fill`, then at the next cell `partial_block(filled_cells.fract())` (also in `seg.fill`) if `Some`, then `░` track in `pal.dim` for the rest. The `pct` text lines stay as they are (they already render the `pct` field — now explicitly the target).

Extend `AgentView` with `ctx_frac: f32`, `shr_frac: f32`, `flash_t: f32`. In `row_cells`, build the two `Seg`s with `frac: v.ctx_frac` / `frac: v.shr_frac` (fill colors unchanged: `fill_color(ctx_frac)` for ctx — feed it the eased frac — and `crate::palette::accent()` for shr). Thread `now: u64` into `row_cells(views, cols, start_row, lay, now)` (unused until Task 4 — name it `now` not `_now`; Task 4 uses it, and the compiler warning in between is acceptable within this task only if committed together — it is not: add `let _ = now;` at the end of `row_cells`, which Task 4 removes).

In `chatview.rs` `agent_views` (~line 14), populate the new fields:

```rust
                let now = crate::anim::now_ms();
                // (hoist `let now` above the .map() closure, once per call)
                crate::chatchips::AgentView {
                    name: a.name.clone(),
                    state: self.agent_state_str(&a.name, active),
                    tok: self.anim.tok(&a.name, now).round() as u64,
                    ctx_pct,
                    share_pct,
                    ctx_frac: self.anim.ctx(&a.name, now),
                    shr_frac: self.anim.shr(&a.name, now),
                    flash_t: self.anim.flash_t(&a.name, now),
                    active,
                }
```

EXCEPTION — tok fallback: when the anim store has no tok entry yet (fresh roster restored from a session), fall back to the raw `ctx` value so restored sessions don't show 0: `let tok_eased = self.anim.tok(&a.name, now); let tok = if tok_eased > 0.0 { tok_eased.round() as u64 } else { ctx };`. Use that `tok`.

Update both `row_cells(...)` call sites (`chatview.rs` — grep `row_cells(`) to pass `crate::anim::now_ms()` as `now`. Update every existing chatchips test constructing `AgentView` to set the three new fields (`ctx_frac: ctx.map(|p| p as f32 / 100.0).unwrap_or(0.0)`, same for shr, `flash_t: 0.0`) so existing alignment assertions run unchanged.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: all pass (existing alignment/level tests untouched in assertions).

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatchips.rs crates/crew-app/src/chatview.rs
git commit -m "feat(crew): eased roster bars with sub-cell left-eighth fill"
```

---

### Task 4: Active-agent pulse + handoff flash rendering

**Files:**
- Modify: `crates/crew-app/src/chatchips.rs` (`row_cells` colors; tests)

**Interfaces:**
- Consumes: `AgentView.flash_t`, `active`, `now` param (Task 3), `anim::{tri, lerp_rgb}`, `crate::palette::accent()`.
- Produces: final row color behavior — pulse while active, flash on handoff — verified by tests.

- [ ] **Step 1: Write the failing tests**

Append to chatchips tests:

```rust
    #[test]
    fn active_agent_pulses_toward_accent() {
        // At tri peak (now = period/2 = 800) an active row's name color must
        // differ from an idle row's; at amplitude ≤ 0.25 it must not reach
        // the accent itself.
        let accent = crate::palette::accent();
        let quiet = name_fg(false, 0.0, 0);      // helper below
        let peak = name_fg(true, 0.0, 800);
        assert_ne!(quiet, peak, "active row breathes");
        assert_ne!(peak, accent, "amplitude stays subtle");
        let trough = name_fg(true, 0.0, 0);
        assert_eq!(trough, quiet, "tri trough = base color");
    }

    #[test]
    fn handoff_flash_blends_and_expires() {
        let fresh = name_fg(false, 1.0, 0);
        let gone = name_fg(false, 0.0, 0);
        assert_ne!(fresh, gone, "fresh flash tints the row");
    }
```

With a `name_fg(active: bool, flash_t: f32, now: u64) -> (u8, u8, u8)` test helper that builds one `AgentView` (reuse the file's `v(...)` fixture, overriding `active`/`flash_t`), runs `layout` + `row_cells(&views, 80, 0, &lay, now)`, and returns the fg of the first cell (the marker) — mirroring how existing tests index cells.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app chatchips`
Expected: FAIL — `active_agent_pulses_toward_accent` (colors equal; no pulse applied yet).

- [ ] **Step 3: Implement**

In `row_cells`, replace the name-color line (`let color = agent_color(&v.name);`) with:

```rust
        // Working agent breathes toward the accent (≤25% blend, 1600ms
        // triangle); a fresh handoff flashes the row (≤35%, 400ms fade).
        // Flash wins over pulse when both apply — it's the newer signal.
        let base = agent_color(&v.name);
        let color = if v.flash_t > 0.0 {
            crate::anim::lerp_rgb(base, crate::palette::accent(), 0.35 * v.flash_t)
        } else if v.active {
            crate::anim::lerp_rgb(
                base,
                crate::palette::accent(),
                0.25 * crate::anim::tri(now, 1600),
            )
        } else {
            base
        };
```

Remove the `let _ = now;` placeholder from Task 3. The `color` feeds the existing marker+name `place(...)` call unchanged.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/chatchips.rs
git commit -m "feat(crew): active-agent pulse + handoff flash on roster rows"
```

---

### Task 5: Redraw tail — anim_active wiring + live verification

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (add `anim_active`)
- Modify: `crates/crew-app/src/paneview.rs:100-106` (`pane_animating`)
- Test: `crates/crew-app/src/chat_tests.rs` (append)

**Interfaces:**
- Consumes: `RosterAnim::active` (Task 1), absorb wiring (Task 2).
- Produces: `ChatPane::anim_active(&self, now: u64) -> bool`; `pane_animating` returns true while roster animation is in flight, so `poll.rs`'s existing busy branch (poll.rs:206-218) keeps redrawing at ~15fps until settle.

- [ ] **Step 1: Write the failing test**

Append to `chat_tests.rs`:

```rust
#[test]
fn anim_active_tail_ends_after_settle() {
    let mut c = test_pane();
    let now = crate::anim::now_ms();
    assert!(!c.anim_active(now), "fresh pane is inactive");
    c.absorb_stats(100, "planner".into(), 50, 5_000);
    assert!(c.anim_active(crate::anim::now_ms()), "ease in flight");
    let after = crate::anim::now_ms() + crate::chatanim::TOK_MS + 1;
    assert!(!c.anim_active(after), "settled → no redraws");
}
```

(Match the fixture + `absorb_stats` parameter order exactly as in Task 2's tests.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p crew-app anim_active_tail`
Expected: FAIL to compile — no method `anim_active`.

- [ ] **Step 3: Implement**

In `chat.rs` (impl ChatPane, near `status_rows`):

```rust
    /// Whether roster animation is mid-flight — drives the redraw tail after
    /// a turn ends so eases/flashes finish, then redraws stop entirely.
    pub(crate) fn anim_active(&self, now: u64) -> bool {
        self.anim.active(now)
    }
```

In `paneview.rs` `pane_animating` (line 100):

```rust
pub(crate) fn pane_animating(p: &Pane) -> bool {
    pane_busy(p)
        || match &p.content {
            PaneContent::Chat(c) => {
                c.is_fading() || c.anim_active(crate::anim::now_ms())
            }
            _ => false,
        }
}
```

- [ ] **Step 4: Run the full suite**

Run: `cargo fmt && cargo test -p crew-app && cargo test -p crew-render`
Expected: all pass, no new warnings (`cargo check -p crew-app 2>&1 | grep -c "^warning"` unchanged from before this plan).

- [ ] **Step 5: Live verification (controller or implementer with GUI access)**

Per `.claude/skills/verify` — build, launch isolated with `HOME=$SCRATCH/home` (symlink `~/Library/Fonts` first!), `CREW_BROKER_MOCK_REPLY="mock"`, open the crew pane (`/crew` in the input bar), send a message, and capture 3 screenshots ~200ms apart during the reply. Expected: the replying agent's bar sweeps (different fill between shots), its marker breathes, and the handoff row flashes; after the turn, one more shot ≥1s later must be identical to a shot taken right before it (no idle repaints — compare pixel-equal).

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/chat.rs crates/crew-app/src/paneview.rs crates/crew-app/src/chat_tests.rs
git commit -m "feat(crew): roster animation redraw tail via pane_animating"
```
