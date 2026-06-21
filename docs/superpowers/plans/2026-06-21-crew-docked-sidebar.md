# Crew Docked Left Sidebar + Top Bar Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn the system-stats view from an auto-tiled grid pane into a **special
docked left sidebar** — fixed width, full height, always on the left — with a **top
bar** containing a toggle icon to show/hide it. Grid panes tile in the remaining
content area.

**Architecture:** The sidebar and top bar are NOT in `self.panes`; they are extra
`PaneScene`s positioned at computed fixed rects and rendered through the existing
`CellView`/`PaneScene` path (no new primitive layer, no overlay). `CrewApp` owns a
`sidebar: Box<StatsPane>` (the stats content, extensible later) and uses the existing
`config.show_nav` (visible flag) + `config.nav_width` (fixed width). A new
`chrome.rs` computes the layout rects + renders the top bar cells + does toggle
hit-testing. `pane_rects` runs over the reserved content sub-rect. The previous
`PaneContent::Stats` grid variant and `spawn_stats_pane` are REMOVED.

**Tech Stack:** Rust 2021; existing crates only (no new deps).

## Global Constraints
- **`crew-render` untouched** (top bar + sidebar are `PaneScene`s of `CellView`s).
- **Every `.rs` ≤ 200 lines (HARD).** `handler.rs` is already near the cap — this plan
  extracts frame-building into `render.rs` to make room.
- **Reserved keys = Super-chords only.** `Cmd+G` now TOGGLES the sidebar.
- `cargo clippy --workspace --all-targets` ZERO warnings; **no `#[allow]`**.
- Layout is in physical px (surface is physical): `nav_px = config.nav_width * scale`,
  `top_bar_h = cell_h + PAD`. Clicks/hit-tests use physical px (the cursor already is).
- Gate: compile + clippy clean + `cargo test` + `timeout 6 cargo run -p crew-app` → exit 124.

---

### Task 1: chrome layout + top-bar cells (`chrome.rs`) (TDD)

**Files:** Create `crates/crew-app/src/chrome.rs`; declare `mod chrome;`.

**Interfaces:**
```rust
use crate::layout::Rect;
use crew_render::CellView;
pub const TOP_PAD: f32 = 10.0;          // extra px added to cell_h for the bar height
pub fn top_bar_h(cell_h: f32) -> f32 { cell_h + TOP_PAD }
pub fn topbar_rect(sw: f32, bar_h: f32) -> Rect;             // (0,0,sw,bar_h)
pub fn sidebar_rect(sw: f32, sh: f32, bar_h: f32, nav_px: f32) -> Rect; // (0,bar_h,nav_px,sh-bar_h)
pub fn content_rect(sw: f32, sh: f32, bar_h: f32, show_nav: bool, nav_px: f32) -> Rect;
pub fn toggle_rect(bar_h: f32) -> Rect;                      // square at top-left, side=bar_h
pub fn point_in(r: Rect, x: f32, y: f32) -> bool;
// top bar content as cells: toggle glyph + title, sized to the bar's cols.
pub fn topbar_cells(show_nav: bool, cols: u16) -> Vec<CellView>;
```
- `content_rect`: `x = show_nav ? nav_px : 0.0`, `y = bar_h`, `w = sw - x`, `h = sh - bar_h`.
- `topbar_cells`: row 0 — col 0 a toggle glyph (`'☰'`, accent `(0,255,160)`), then a
  space, then `"Crew"` in `(200,200,200)`; bg `(8,8,16)`; clamp to `cols`.
  (The glyph indicates the toggle; the click target is `toggle_rect`.)

- [ ] **Step 1: Failing tests** (`chrome.rs` `#[cfg(test)]`):
  (a) `content_rect(1000, 800, 30.0, false, 200.0)` → `Rect{x:0,y:30,w:1000,h:770}`;
  (b) `content_rect(1000, 800, 30.0, true, 200.0)` → `Rect{x:200,y:30,w:800,h:770}`;
  (c) `sidebar_rect(1000,800,30.0,200.0)` → `Rect{x:0,y:30,w:200,h:770}`;
  (d) `point_in(toggle_rect(30.0), 5.0, 5.0)` true; `point_in(toggle_rect(30.0), 100.0, 5.0)` false;
  (e) `topbar_cells(false, 20)` is non-empty, all `row==0`, first cell `c=='☰'`.
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement.** Keep chrome.rs ≤200.
- [ ] **Step 4: Run → pass;** `cargo test -p crew-app chrome` green; clippy clean.
- [ ] **Step 5: Commit** — `feat(crew-app): chrome layout rects + top-bar cells`.

---

### Task 2: remove grid Stats; add docked sidebar state + frame builder (`render.rs`)

**Files:** Modify `pane.rs`, `app.rs`, `statspane.rs`; create `render.rs`; modify `handler.rs`.

**Interfaces:**
- `pane.rs`: REMOVE `PaneContent::Stats(Box<StatsPane>)` and its `Pane::cells` arm.
- `app.rs`: REMOVE `spawn_stats_pane`; add field `sidebar: Box<StatsPane>` to `CrewApp`
  (construct in `run()` — `CrewApp { config: …, sidebar: Box::new(StatsPane::new()), ..Default::default() }`; `StatsPane` is not `Default`-derivable on the struct literal path, so build it explicitly). Change `handle_super_chord` `"g"` to
  `"g" => { self.config.show_nav = !self.config.show_nav; self.config.save(); }`.
  Add `pub(crate) fn toggle_sidebar(&mut self)` doing the same (used by the mouse).
- `render.rs` (new): `impl CrewApp { pub(crate) fn build_frame(&mut self) -> Vec<PaneScene> }`
  — moves the body currently in `handler.rs`'s `RedrawRequested`:
  1. get `cell_w,cell_h,sw,sh`; `scale = self.window…scale_factor()`;
  2. `bar_h = chrome::top_bar_h(cell_h)`; `nav_px = self.config.nav_width * scale`;
  3. `c = chrome::content_rect(sw,sh,bar_h,self.config.show_nav,nav_px)`;
  4. grid rects = `layout::pane_rects_at(n, c.x, c.y, c.w, c.h, GAP)` (NEW fn — see
     below); `relayout(&mut self.panes, &rects, cell_w, cell_h)`;
  5. `scenes = build_scenes(&self.panes, self.focused)`;
  6. push a top-bar `PaneScene { cells: chrome::topbar_cells(show_nav, cols_from(topbar_rect)), x,y,w,h from topbar_rect, focused:false }`;
  7. if `show_nav`, push a sidebar `PaneScene` with `cells = self.sidebar.cells(side_cols, side_rows)` from `sidebar_rect`;
  8. return all scenes.
- `layout.rs`: add `pub fn pane_rects_at(n, ox: f32, oy: f32, w: f32, h: f32, gap: f32)
  -> Vec<Rect>` (the existing `pane_rects` logic over `(w,h)` then each rect `+= (ox,oy)`);
  refactor `pane_rects(n,w,h,gap)` to call `pane_rects_at(n,0.0,0.0,w,h,gap)` so the
  existing tests still pass. `cols_from(rect)` = `(rect.w / cell_w).floor() as u16`.
- `handler.rs`: `RedrawRequested` becomes `let scenes = self.build_frame(); if let
  Some(r) = &mut self.renderer { r.frame(&scenes); }` (guard empty/zero as before).
  In `about_to_wait`, REMOVE the `PaneContent::Stats` arm; AFTER the pane loop add
  `if self.sidebar.refresh() { any_changed = true; }`. REMOVE the
  `PaneContent::Stats(_) => {}` key-routing arm.

- [ ] **Step 1:** Remove the grid Stats variant + spawn + arms (compile errors guide you).
- [ ] **Step 2:** Add `pane_rects_at` (+ keep `pane_rects` delegating; run
  `cargo test -p crew-app layout` — existing tests must still pass).
- [ ] **Step 3:** Add `sidebar` field + `toggle_sidebar` + `"g"` toggle; build `render.rs`.
- [ ] **Step 4:** Rewire `handler.rs` (RedrawRequested → `build_frame`; sidebar
  refresh in `about_to_wait`).
- [ ] **Step 5: Gate** — build; clippy `--workspace --all-targets` ZERO; `cargo test
  --workspace` green; EVERY `.rs` ≤200 (split further if needed); `timeout 6 cargo run
  -p crew-app` exit 124. Manual: top bar shows `☰ Crew`; sidebar docked left with
  gauges; `Cmd+G` toggles it; grid panes fill the area right of the sidebar.
- [ ] **Step 6: Commit** — `feat(crew-app): docked left stats sidebar + top bar`.

---

### Task 3: toggle-icon click + focus guard (build-run-observe)

**Files:** Modify `handler.rs` (or extract mouse handling to keep ≤200).

- [ ] **Step 1:** In `MouseInput` Pressed: compute `cell_h`,`sw`,`sh`; `bar_h =
  chrome::top_bar_h(cell_h)`. If `chrome::point_in(chrome::toggle_rect(bar_h),
  cursor.0, cursor.1)` → `self.toggle_sidebar(); self.redraw(); return;`.
- [ ] **Step 2:** Otherwise only change pane focus when the click is inside the
  content rect: compute `c = chrome::content_rect(...)`; gate the existing
  `pane_at` focus assignment behind `chrome::point_in(c, cursor.0, cursor.1)`. Use
  `pane_rects_at(n, c.x, c.y, c.w, c.h, GAP)` for the hit-test so it matches the
  rendered grid offsets.
- [ ] **Step 3: Gate** — build; clippy ZERO; tests green; `.rs` ≤200; launch exit 124.
  Manual: clicking `☰` shows/hides the sidebar; clicking a grid pane focuses it;
  clicking the top bar / sidebar does not steal focus.
- [ ] **Step 4: Commit** — `feat(crew-app): top-bar toggle click + content-area focus guard`.

---

### Task 4: cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` ZERO warnings.
- [ ] **Step 2:** `cargo test --workspace` all green.
- [ ] **Step 3:** Every `.rs` in `crates/crew-*` ≤200.
- [ ] **Step 4:** Manual smoke (record in commit): docked sidebar gauges; `☰`/`Cmd+G`
  toggle + persist; grid tiling right of sidebar; `Cmd+T/J/O/W/M/Q`, `Cmd+,` settings,
  click-focus all still work; resize + maximize reflow correctly.
- [ ] **Step 5:** Commit milestone — `chore: Crew docked sidebar + top bar milestone`.

## Notes
- The sidebar is a container; `StatsPane` is its current content. Later sections
  (agent list, file tree) compose into the same sidebar rect — extend `sidebar.cells`.
- `nav_width` is now load-bearing (sidebar width); editable from the Settings pane.
