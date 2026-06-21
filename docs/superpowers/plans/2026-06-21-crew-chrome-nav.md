# Crew Top Bar + Collapsible Left Nav (gauges) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add an in-app top bar with a nav-toggle button and a collapsible left
navigation panel showing live CPU / memory / disk usage as bar gauges. Panes tile
into the remaining content area.

**Architecture:** `crew-render` gains a tiny immediate-mode primitive API
(`UiRect` filled rect + `UiText` positioned label) drawn on top of the pane scene;
it stays ignorant of config/stats/panes. `crew-app` computes all chrome geometry in
`chrome.rs`, samples the system in `stats.rs` (via `sysinfo`), reserves the content
sub-rect for `pane_rects`, and routes the toggle-button click. `config.show_nav`
(already exists) holds the collapsed/expanded state; `config.nav_width` the width.

**Tech Stack:** Rust 2021; existing crates + `sysinfo` (new workspace dep).

## Global Constraints
- **`crew-render` imports neither `crew-term` nor `crew-plugin`** and knows nothing of config/stats.
- **Every `.rs` ≤ 200 lines (HARD).**
- **Reserved keys = Super-chords only;** non-Super keys go to the focused pane.
- `cargo clippy --workspace --all-targets` ZERO warnings; **no `#[allow]`**.
- The top bar is **always** visible (it hosts the toggle); the nav reserves width
  only when `config.show_nav`.
- Gate: compile + clippy clean + `cargo test` + `timeout 6 cargo run -p crew-app` → exit 124.

---

### Task 1: crew-render primitive API — `UiRect` / `UiText` (build-run-observe)

**Files:**
- Create `crates/crew-render/src/ui.rs` (`UiRect`, `UiText`).
- Modify `crates/crew-render/src/celltext.rs` (add `build_label_buffer`).
- Modify `crates/crew-render/src/cellgrid.rs` (append chrome quads + label buffers in a new `set_chrome`, or extend `set_scene`).
- Modify `crates/crew-render/src/renderer.rs` (`frame` signature).
- Modify `crates/crew-render/src/lib.rs` (`pub use ui::{UiRect, UiText};`).
- Modify `crates/crew-app/src/handler.rs` (update the single `frame` call site).

**Interfaces:**
```rust
// ui.rs
pub struct UiRect { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub color: [f32; 4] }
pub struct UiText { pub text: String, pub x: f32, pub y: f32, pub color: (u8, u8, u8) }
// renderer.rs
pub fn frame(&mut self, panes: &[PaneScene], rects: &[UiRect], texts: &[UiText])
// celltext.rs — a single-line label buffer whose glyphs carry `color` via Attrs.
pub fn build_label_buffer(fs: &mut FontSystem, text: &str, color: (u8, u8, u8), params: &FontParams) -> Buffer
```
Drawing order inside `frame`: pane bg quads + borders (existing) → **chrome `rects`
as quads (appended to the same `QuadLayer`)** → pane text (existing) → **chrome
label buffers (appended to the text areas)**. Chrome rects draw over pane
backgrounds; chrome labels render on top. Implementation detail: extend the
internal scene build so `quads` gets the `UiRect`s mapped to `Quad` and
`pane_buffers` gets one `(label_buf, x, y, big_w, big_h)` per `UiText` with generous
bounds (e.g. `w=4096`) so labels are not clipped.

- [ ] **Step 1:** Create `ui.rs` with the two structs (derive `Clone`, `Debug`).
  Export from `lib.rs`.
- [ ] **Step 2:** Add `build_label_buffer` to celltext.rs — build a `Buffer` with
  `Metrics::new(params.font_size, params.line_height)`, `Wrap::None`, set one rich
  text span `text` with `Attrs::new().color(Color::rgb(color.0, color.1, color.2))`.
  Keep celltext.rs ≤200 (it is ~143 now; if the addition crosses, move `cell_metrics`
  or the probe helper into a new `metrics.rs`).
- [ ] **Step 3:** In cellgrid.rs, change `set_scene` to also accept
  `rects: &[UiRect]` and `texts: &[UiText]` (or add `set_chrome` called right after
  `set_scene`). Append: for each `UiRect` push a `Quad { x,y,w,h,color }`; for each
  `UiText` push `(build_label_buffer(...), x, y, 4096.0, params.line_height + 4.0)` to
  the pane-buffers list. If cellgrid.rs would exceed 200, factor the chrome-append
  into a free fn in scene.rs. Update `Renderer::frame` to forward the new slices.
- [ ] **Step 4:** Update `handler.rs` `RedrawRequested`: call
  `renderer.frame(&scenes, &[], &[])` for now (empty chrome — real chrome is Task 4).
- [ ] **Step 5: Gate** — `cargo build -p crew-app`; `cargo clippy --workspace
  --all-targets` ZERO warnings; `cargo test --workspace` green; all touched `.rs`
  ≤200; `timeout 6 cargo run -p crew-app` exit 124 (panes still render unchanged).
- [ ] **Step 6: Commit** — `feat(crew-render): UiRect/UiText chrome primitive API`.

---

### Task 2: system stats sampler (`stats.rs`) (TDD)

**Files:**
- Create `crates/crew-app/src/stats.rs`; declare `mod stats;` in the crate root.
- Modify root `Cargo.toml` `[workspace.dependencies]`: add `sysinfo = "0.33"`.
- Modify `crates/crew-app/Cargo.toml`: add `sysinfo = { workspace = true }`.

**Interfaces:**
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats { pub cpu: f32, pub mem: f32, pub disk: f32 }  // each 0.0..=1.0
pub struct SysSampler { /* sysinfo::System + disks + last sample tick */ }
impl SysSampler {
    pub fn new() -> Self;
    pub fn stats(&self) -> Stats;                 // last sampled
    pub fn maybe_refresh(&mut self, now_ms: u128); // re-sample only past throttle
}
// pure helper, testable without sysinfo:
pub fn fraction(used: u64, total: u64) -> f32;    // total==0 -> 0.0; clamp 0..=1
```
`maybe_refresh` stores `last_ms`; re-samples only when `now_ms - last_ms >= 1000`.
CPU% via `sysinfo` global usage / 100; mem via `used_memory`/`total_memory`; disk via
summed used/total across disks. Sampling needs `refresh_cpu_usage` etc.; the throttle
keeps it off the 16ms tick.

- [ ] **Step 1: Failing tests** (`stats.rs` `#[cfg(test)]`):
  (a) `fraction(0, 0) == 0.0`; (b) `fraction(50, 100) == 0.5`;
  (c) `fraction(200, 100) == 1.0` (clamped);
  (d) a `SysSampler::new()` then `maybe_refresh(0)` then `maybe_refresh(500)` does
  NOT change `last_ms` past the first (throttle holds) while `maybe_refresh(2000)`
  does — assert via a test hook: expose `pub(crate) last_ms` or a
  `pub fn last_ms(&self) -> u128` so the test can observe throttle behaviour without
  real sampling. (Keep the real sampling out of the unit test — only the throttle
  bookkeeping and `fraction` are asserted.)
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** Add deps; implement. `Default for Stats` is derived. `new()`
  constructs the `System`, does one initial refresh so `stats()` is non-garbage.
- [ ] **Step 4: Run → pass;** `cargo test -p crew-app stats` green; clippy clean;
  stats.rs ≤200.
- [ ] **Step 5: Commit** — `feat(crew-app): sysinfo system stats sampler`.

---

### Task 3: chrome geometry (`chrome.rs`) (TDD)

**Files:** Create `crates/crew-app/src/chrome.rs`; declare `mod chrome;`.

**Interfaces:**
```rust
use crate::layout::Rect;
pub const TOP_BAR_H: f32 = 30.0;
pub const TOGGLE_W: f32 = 36.0;
// content region left for panes after reserving top bar (+ nav if shown):
pub fn content_rect(surface_w: f32, surface_h: f32, show_nav: bool, nav_w: f32) -> Rect;
// hit-rect of the toggle button (top-left of the bar):
pub fn toggle_rect() -> Rect;
pub fn point_in(r: Rect, x: f32, y: f32) -> bool;
// build the chrome primitives for the current frame:
pub fn build_chrome(surface_w: f32, surface_h: f32, show_nav: bool, nav_w: f32,
                    stats: crate::stats::Stats, cell_w: f32, cell_h: f32)
    -> (Vec<crew_render::UiRect>, Vec<crew_render::UiText>);
```
`content_rect`: `x = show_nav ? nav_w : 0`, `y = TOP_BAR_H`,
`w = surface_w - x`, `h = surface_h - TOP_BAR_H`. `build_chrome` emits: a top-bar
`UiRect` (full width × TOP_BAR_H), a toggle-button `UiRect` + a `UiText` glyph
(`"≡"`), a "Crew" title `UiText`; and when `show_nav`, a nav-background `UiRect` plus
three gauges (CPU/MEM/DISK), each = a track `UiRect`, a fill `UiRect`
(`width = nav_inner * frac`), and a `UiText` label like `"CPU  42%"`. Use the neon
palette (bar fill `(0,255,160)`, track `(40,80,95)`, bar bg `(8,8,16)`). Positions in
**physical px** (caller passes already-scaled values); the label baseline offsets use
`cell_h`.

- [ ] **Step 1: Failing tests** (`chrome.rs` `#[cfg(test)]`):
  (a) `content_rect(1000, 800, false, 210)` → `Rect{ x:0, y:30, w:1000, h:770 }`;
  (b) `content_rect(1000, 800, true, 210)` → `Rect{ x:210, y:30, w:790, h:770 }`;
  (c) `point_in(toggle_rect(), 5.0, 5.0)` is true and `point_in(toggle_rect(),
  500.0, 5.0)` is false;
  (d) `build_chrome(1000,800,false,210,Stats::default(),8.0,18.0)` returns at least
  one `UiRect` (top bar) and the toggle label, and **no** gauge rects (nav hidden);
  (e) `build_chrome(1000,800,true,210, Stats{cpu:0.5,mem:0.0,disk:0.0}, 8.0,18.0)`
  returns more rects than the hidden case, and the CPU fill rect width is ≈ half the
  gauge track width.
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3:** Implement. Keep chrome.rs ≤200 — if the gauge builder is large,
  split a `gauges.rs` helper and re-export. No `#[allow]`.
- [ ] **Step 4: Run → pass;** `cargo test -p crew-app chrome` green; clippy clean; ≤200.
- [ ] **Step 5: Commit** — `feat(crew-app): chrome geometry + gauge primitives`.

---

### Task 4: wire chrome into the app + toggle click (build-run-observe)

**Files:** Modify `crates/crew-app/src/app.rs`, `crates/crew-app/src/handler.rs`.

**Interfaces:** `CrewApp` gains `pub(crate) sampler: SysSampler` (field; construct in
`run()` like `config`). Pane layout now uses the content rect.

- [ ] **Step 1: Reserve content area.** Where `RedrawRequested` and the mouse handler
  call `pane_rects(n, sw, sh, GAP)`, change to compute `let c =
  chrome::content_rect(sw, sh, self.config.show_nav, self.config.nav_width)` then
  `pane_rects` over `(c.w, c.h)` translated by `(c.x, c.y)` — add
  `layout::pane_rects_at(n, origin_x, origin_y, w, h, gap)` (new fn; keep the old one
  delegating to it with origin 0,0 so existing tests stand). Update BOTH the redraw
  path and the click hit-test path to use the same offset rects.
- [ ] **Step 2: Draw chrome.** In `RedrawRequested`, after `build_scenes`, call
  `self.sampler.maybe_refresh(now_ms)` (derive `now_ms` from `Instant`/a monotonic
  counter — reuse the existing `Instant::now()` already imported; convert to millis
  via `elapsed` from a stored start, OR store a `frame_ms` counter incremented per
  tick — simplest: keep a `start: Instant` in `CrewApp` set in `resumed`, pass
  `start.elapsed().as_millis()`), then
  `let (rects, texts) = chrome::build_chrome(sw, sh, show_nav, nav_w,
  self.sampler.stats(), cell_w, cell_h);` and `renderer.frame(&scenes, &rects, &texts)`.
- [ ] **Step 3: Toggle click.** In the `MouseInput` Pressed handler, BEFORE pane
  hit-testing, if `chrome::point_in(chrome::toggle_rect(), cursor.x, cursor.y)` →
  flip `self.config.show_nav`, `redraw`, and `return` (do not change focus). Also: a
  click anywhere in the top bar or nav region (y < TOP_BAR_H, or x < nav_w when
  shown) must NOT change pane focus — guard the focus assignment with the content
  rect (`point_in(content_rect, x, y)`).
- [ ] **Step 4: Keep stats moving.** Ensure `about_to_wait` still requests redraws on
  pane changes; the gauges refresh at most ~1/s via the throttle, repainting on the
  next redraw. (No busy loop.)
- [ ] **Step 5: Gate** — `cargo build -p crew-app`; clippy ZERO; `cargo test
  --workspace` green; all `.rs` ≤200; `timeout 6 cargo run -p crew-app` exit 124.
  Manual: top bar visible; clicking the toggle shows/hides the left nav; gauges show
  CPU/MEM/DISK; panes fill the area to the right of the nav and below the bar.
- [ ] **Step 6: Commit** — `feat(crew-app): top bar + collapsible left nav with gauges`.

---

### Task 5: cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` ZERO warnings.
- [ ] **Step 2:** `cargo test --workspace` all green.
- [ ] **Step 3:** Every `.rs` in `crates/crew-*` ≤200.
- [ ] **Step 4:** Manual smoke (record in commit): toggle button collapses/expands the
  nav; CPU/MEM/DISK gauges animate; terminal panes, `Cmd+T/J/O/W`, `Cmd+M`, `Cmd+Q`
  all still work; resize reflows content beside the nav.
- [ ] **Step 5:** Commit milestone — `chore: Crew top bar + left nav gauges milestone`.

## Notes for the next phase (Plan C — settings overlay)
- The settings overlay reuses `UiRect`/`UiText` (drawn last, on top).
- `Cmd+,` opens it; keyboard adjust mutates `CrewConfig`; live `set_font_size`;
  `config.save()` on close. Font size + nav width + show_nav become editable there.
