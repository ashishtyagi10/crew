# Agent Grid Layout — Implementation Plan (Plan 1 of 4)

> *Historical record: this plan predates the Crew pivot and targets editor crates that have since been removed.*

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a tested `grid` module to `legacy-core` that lays N agent tiles out in a near-square grid, caps the full tiles at 6, and demotes the least-recently-active tiles into a minimized thumbnail strip (LRU).

**Architecture:** A pure, UI-independent `legacy-core::grid` module. `GridLayout` holds tile ids in most-recently-active-first order; `grid_rects` packs a count of tiles into a near-square grid; `compute_grid_layout` combines them — reserving a bottom strip for minimized tiles and gridding the rest. This plan is **purely additive**: it builds and unit-tests the engine without touching the live render path, so the running app is unchanged. Wiring into rendering happens in Plan 2.

**Tech Stack:** Rust, `ratatui::layout::Rect` (already a `legacy-core` dependency), standard `cargo test`.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, no exceptions.
- Match existing `legacy-core` module style: a `mod.rs` that declares submodules and re-exports the public API, with `#[cfg(test)] mod tests;` (mirror `panel_layout/mod.rs`).
- `lib.rs` uses glob re-exports (`pub use grid::*;`). Public types must be unambiguously named (no collision with existing `LayoutNode`/`PanelLeaf`).
- Near-square grid rule: `cols = ceil(sqrt(count))`, `rows = ceil(count / cols)`, row-major fill.
- Full-tile cap: `MAX_FULL_TILES = 6`. Tiles beyond the 6 most-recently-active are minimized.
- Tile ids are `usize` (same id space as `App::terminals` indices / `PanelLeaf::Terminal(usize)`).

---

### Task 1: Grid geometry — `grid_rects`

Pack `count` tiles into a near-square grid within an area. Pure function, no state.

**Files:**
- Create: `crates/legacy-core/src/grid/mod.rs`
- Create: `crates/legacy-core/src/grid/geometry.rs`
- Create: `crates/legacy-core/src/grid/tests.rs`
- Modify: `crates/legacy-core/src/lib.rs` (add `pub mod grid;` and `pub use grid::*;`)

**Interfaces:**
- Produces: `pub fn grid_rects(area: ratatui::layout::Rect, count: usize) -> Vec<ratatui::layout::Rect>` — returns exactly `count` rects (empty when `count == 0`), row-major (top-to-bottom, left-to-right), tiling `area` with no overlaps; the last row stretches its tiles to fill the row width.
- Produces (constant): `pub const MAX_FULL_TILES: usize = 6;`

- [ ] **Step 1: Create the module skeleton and wire it into the crate**

Create `crates/legacy-core/src/grid/mod.rs`:

```rust
//! Agent grid layout: pack N tiles into a near-square grid and track which
//! tiles are shown full vs. minimized (LRU). UI-independent; consumed by the
//! renderer.

mod geometry;

#[cfg(test)]
mod tests;

pub use geometry::{grid_rects, MAX_FULL_TILES};
```

Modify `crates/legacy-core/src/lib.rs` — add the module declaration after the `panel_layout` line and a glob re-export after the `panel_layout` re-export:

```rust
pub mod action;
pub mod config;
pub mod error;
pub mod grid;
pub mod keymap;
pub mod panel_layout;
pub mod tab_group;
pub mod tree;
pub mod types;
pub mod update;

pub use action::Action;
pub use config::AppConfig;
pub use error::LegacyError;
pub use grid::*;
pub use keymap::KeyMap;
pub use panel_layout::*;
pub use tab_group::TabGroup;
pub use tree::*;
pub use types::*;
```

- [ ] **Step 2: Write the failing tests**

Create `crates/legacy-core/src/grid/tests.rs`:

```rust
use super::*;
use ratatui::layout::Rect;

fn area() -> Rect {
    Rect::new(0, 0, 120, 40)
}

#[test]
fn grid_rects_zero_is_empty() {
    assert!(grid_rects(area(), 0).is_empty());
}

#[test]
fn grid_rects_one_fills_area() {
    let r = grid_rects(area(), 1);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], area());
}

#[test]
fn grid_rects_two_side_by_side() {
    let r = grid_rects(area(), 2);
    assert_eq!(r.len(), 2);
    // One row, two columns: same height as the area, each ~half width.
    assert_eq!(r[0].height, 40);
    assert_eq!(r[1].height, 40);
    assert_eq!(r[0].x, 0);
    assert!(r[1].x >= 59 && r[1].x <= 61);
    // No horizontal overlap.
    assert!(r[0].x + r[0].width <= r[1].x);
}

#[test]
fn grid_rects_four_is_two_by_two() {
    let r = grid_rects(area(), 4);
    assert_eq!(r.len(), 4);
    // Two distinct row y-offsets, two distinct column x-offsets.
    let ys: std::collections::BTreeSet<u16> = r.iter().map(|x| x.y).collect();
    let xs: std::collections::BTreeSet<u16> = r.iter().map(|x| x.x).collect();
    assert_eq!(ys.len(), 2);
    assert_eq!(xs.len(), 2);
}

#[test]
fn grid_rects_six_is_three_cols_two_rows() {
    let r = grid_rects(area(), 6);
    assert_eq!(r.len(), 6);
    let ys: std::collections::BTreeSet<u16> = r.iter().map(|x| x.y).collect();
    let xs: std::collections::BTreeSet<u16> = r.iter().map(|x| x.x).collect();
    assert_eq!(xs.len(), 3); // cols = ceil(sqrt(6)) = 3
    assert_eq!(ys.len(), 2); // rows = ceil(6/3) = 2
}

#[test]
fn grid_rects_three_last_row_stretches_full_width() {
    // cols = ceil(sqrt(3)) = 2, rows = 2. Row 0 has 2 tiles, row 1 has 1.
    let r = grid_rects(area(), 3);
    assert_eq!(r.len(), 3);
    // The lone tile on the last row stretches to the full area width.
    let last = r[2];
    assert_eq!(last.x, 0);
    assert_eq!(last.width, 120);
}

#[test]
fn grid_rects_cover_without_overlap_for_many_counts() {
    for count in 1..=12usize {
        let rects = grid_rects(area(), count);
        assert_eq!(rects.len(), count, "count {count}");
        for (i, a) in rects.iter().enumerate() {
            assert!(a.width > 0 && a.height > 0, "count {count} tile {i} empty");
            for b in rects.iter().skip(i + 1) {
                let disjoint = a.x + a.width <= b.x
                    || b.x + b.width <= a.x
                    || a.y + a.height <= b.y
                    || b.y + b.height <= a.y;
                assert!(disjoint, "count {count}: tiles {a:?} and {b:?} overlap");
            }
        }
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p legacy-core grid::`
Expected: FAIL — compile error, `grid_rects` not found / `geometry` module empty.

- [ ] **Step 4: Implement `grid_rects`**

Create `crates/legacy-core/src/grid/geometry.rs`:

```rust
use ratatui::layout::Rect;

/// Maximum number of tiles shown at full size; the rest are minimized.
pub const MAX_FULL_TILES: usize = 6;

/// Pack `count` tiles into a near-square grid within `area`.
///
/// `cols = ceil(sqrt(count))`, `rows = ceil(count / cols)`, filled row-major
/// (top-to-bottom, left-to-right). Row heights split `area.height` evenly
/// (early rows absorb the remainder). Within each row the tiles split that
/// row's width evenly, so a short final row stretches to fill the width.
/// Returns exactly `count` rects; empty when `count == 0`.
pub fn grid_rects(area: Rect, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(cols);

    let mut out = Vec::with_capacity(count);
    let mut remaining = count;
    for row in 0..rows {
        let tiles_in_row = remaining.min(cols);
        let y = area.y + span_start(area.height, rows, row);
        let h = span_len(area.height, rows, row);
        for col in 0..tiles_in_row {
            let x = area.x + span_start(area.width, tiles_in_row, col);
            let w = span_len(area.width, tiles_in_row, col);
            out.push(Rect::new(x, y, w, h));
        }
        remaining -= tiles_in_row;
    }
    out
}

/// Offset of slice `index` when dividing `total` into `parts` near-equal
/// spans (earlier spans absorb the remainder, so coverage is exact).
fn span_start(total: u16, parts: usize, index: usize) -> u16 {
    let base = total / parts as u16;
    let rem = total % parts as u16;
    let i = index as u16;
    base * i + i.min(rem)
}

/// Length of slice `index` when dividing `total` into `parts` near-equal
/// spans (earlier spans get one extra unit until the remainder is spent).
fn span_len(total: u16, parts: usize, index: usize) -> u16 {
    let base = total / parts as u16;
    let rem = total % parts as u16;
    base + if (index as u16) < rem { 1 } else { 0 }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p legacy-core grid::`
Expected: PASS — all 7 tests green.

- [ ] **Step 6: Format, lint, full test**

Run: `cargo fmt && cargo clippy -p legacy-core -- -W clippy::all && cargo test -p legacy-core`
Expected: no new clippy warnings in `grid/`, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/legacy-core/src/grid/mod.rs crates/legacy-core/src/grid/geometry.rs crates/legacy-core/src/grid/tests.rs crates/legacy-core/src/lib.rs
git commit -m "feat(core): near-square grid_rects packing for agent tiles"
```

---

### Task 2: LRU state — `GridLayout`

Track tile ids in most-recently-active-first order; expose which are full vs. minimized.

**Files:**
- Create: `crates/legacy-core/src/grid/state.rs`
- Modify: `crates/legacy-core/src/grid/mod.rs` (declare `mod state;` and re-export)
- Modify: `crates/legacy-core/src/grid/tests.rs` (append state tests)

**Interfaces:**
- Consumes: `MAX_FULL_TILES` from Task 1.
- Produces:
  - `pub struct GridLayout` with `pub fn new() -> Self`, `Default`.
  - `pub fn add(&mut self, id: usize)` — insert as most-recently-active (front); idempotent on id.
  - `pub fn remove(&mut self, id: usize)` — drop the id.
  - `pub fn touch(&mut self, id: usize)` — move an existing id to the front; no-op if absent.
  - `pub fn full(&self) -> &[usize]` — the up-to-`MAX_FULL_TILES` most-recently-active ids.
  - `pub fn minimized(&self) -> &[usize]` — the remaining ids (least-recently-active).
  - `pub fn len(&self) -> usize`, `pub fn is_empty(&self) -> bool`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/legacy-core/src/grid/tests.rs`:

```rust
#[test]
fn gridlayout_add_orders_most_recent_first() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(2);
    // Most-recently-added is first.
    assert_eq!(g.full(), &[2, 1, 0]);
    assert!(g.minimized().is_empty());
    assert_eq!(g.len(), 3);
}

#[test]
fn gridlayout_add_is_idempotent_and_promotes() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(0); // re-add existing -> moves to front, no duplicate
    assert_eq!(g.full(), &[0, 1]);
    assert_eq!(g.len(), 2);
}

#[test]
fn gridlayout_seventh_tile_minimizes_least_recent() {
    let mut g = GridLayout::new();
    for id in 0..7 {
        g.add(id); // 6 added most-recent-first, then a 7th
    }
    // Front six are full; the least-recently-active (id 0) is minimized.
    assert_eq!(g.full(), &[6, 5, 4, 3, 2, 1]);
    assert_eq!(g.minimized(), &[0]);
}

#[test]
fn gridlayout_touch_promotes_into_full_set() {
    let mut g = GridLayout::new();
    for id in 0..7 {
        g.add(id);
    }
    // id 0 is minimized; touching it promotes it and demotes the current LRU (id 1).
    g.touch(0);
    assert_eq!(g.full()[0], 0);
    assert_eq!(g.minimized(), &[1]);
}

#[test]
fn gridlayout_touch_absent_is_noop() {
    let mut g = GridLayout::new();
    g.add(0);
    g.touch(99);
    assert_eq!(g.full(), &[0]);
}

#[test]
fn gridlayout_remove_drops_id() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.remove(1);
    assert_eq!(g.full(), &[0]);
    assert_eq!(g.len(), 1);
}

#[test]
fn gridlayout_empty_state() {
    let g = GridLayout::new();
    assert!(g.is_empty());
    assert_eq!(g.len(), 0);
    assert!(g.full().is_empty());
    assert!(g.minimized().is_empty());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p legacy-core grid::`
Expected: FAIL — `GridLayout` not found.

- [ ] **Step 3: Implement `GridLayout`**

Create `crates/legacy-core/src/grid/state.rs`:

```rust
use super::geometry::MAX_FULL_TILES;

/// Tracks agent tile ids in most-recently-active-first order. The first
/// `MAX_FULL_TILES` are shown full; the rest are minimized (LRU).
#[derive(Debug, Clone, Default)]
pub struct GridLayout {
    /// Tile ids, most-recently-active first.
    order: Vec<usize>,
}

impl GridLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `id` as the most-recently-active tile. If it already exists it
    /// is moved to the front rather than duplicated.
    pub fn add(&mut self, id: usize) {
        self.order.retain(|x| *x != id);
        self.order.insert(0, id);
    }

    /// Remove `id` from the layout, if present.
    pub fn remove(&mut self, id: usize) {
        self.order.retain(|x| *x != id);
    }

    /// Move an existing `id` to the front (most-recently-active). No-op if
    /// `id` is not present.
    pub fn touch(&mut self, id: usize) {
        if let Some(pos) = self.order.iter().position(|x| *x == id) {
            let v = self.order.remove(pos);
            self.order.insert(0, v);
        }
    }

    fn split_point(&self) -> usize {
        self.order.len().min(MAX_FULL_TILES)
    }

    /// Ids shown at full size (the most-recently-active, up to the cap).
    pub fn full(&self) -> &[usize] {
        &self.order[..self.split_point()]
    }

    /// Ids that are minimized (least-recently-active beyond the cap).
    pub fn minimized(&self) -> &[usize] {
        &self.order[self.split_point()..]
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}
```

Modify `crates/legacy-core/src/grid/mod.rs` to add the submodule and re-export:

```rust
//! Agent grid layout: pack N tiles into a near-square grid and track which
//! tiles are shown full vs. minimized (LRU). UI-independent; consumed by the
//! renderer.

mod geometry;
mod state;

#[cfg(test)]
mod tests;

pub use geometry::{grid_rects, MAX_FULL_TILES};
pub use state::GridLayout;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p legacy-core grid::`
Expected: PASS — all state tests green.

- [ ] **Step 5: Format, lint, full test**

Run: `cargo fmt && cargo clippy -p legacy-core -- -W clippy::all && cargo test -p legacy-core`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/legacy-core/src/grid/state.rs crates/legacy-core/src/grid/mod.rs crates/legacy-core/src/grid/tests.rs
git commit -m "feat(core): GridLayout LRU state (6 full, rest minimized)"
```

---

### Task 3: Combined layout — `compute_grid_layout`

Turn a `GridLayout` + an area into concrete rects: full tiles gridded in the main region, minimized tiles laid left-to-right across a reserved bottom strip.

**Files:**
- Create: `crates/legacy-core/src/grid/compose.rs`
- Modify: `crates/legacy-core/src/grid/mod.rs` (declare `mod compose;` and re-export)
- Modify: `crates/legacy-core/src/grid/tests.rs` (append compose tests)

**Interfaces:**
- Consumes: `grid_rects`, `MAX_FULL_TILES`, `GridLayout` from Tasks 1–2.
- Produces:
  - `pub struct GridRects { pub full: Vec<(usize, Rect)>, pub minimized: Vec<(usize, Rect)> }`
  - `pub const MINIMIZED_STRIP_HEIGHT: u16 = 3;`
  - `pub fn compute_grid_layout(area: ratatui::layout::Rect, layout: &GridLayout) -> GridRects` — pairs `layout.full()` ids with grid rects in the main region; when `layout.minimized()` is non-empty, reserves a `MINIMIZED_STRIP_HEIGHT`-row strip at the bottom and lays minimized ids evenly across it. Empty layout → both vecs empty.

- [ ] **Step 1: Write the failing tests**

Append to `crates/legacy-core/src/grid/tests.rs`:

```rust
#[test]
fn compose_empty_layout_is_empty() {
    let g = GridLayout::new();
    let out = compute_grid_layout(area(), &g);
    assert!(out.full.is_empty());
    assert!(out.minimized.is_empty());
}

#[test]
fn compose_no_minimized_uses_full_area() {
    let mut g = GridLayout::new();
    g.add(0);
    let out = compute_grid_layout(area(), &g);
    assert_eq!(out.full.len(), 1);
    assert!(out.minimized.is_empty());
    // With no strip reserved, the single tile fills the whole area height.
    assert_eq!(out.full[0].1.height, 40);
    assert_eq!(out.full[0].0, 0); // id preserved
}

#[test]
fn compose_reserves_strip_when_minimized_present() {
    let mut g = GridLayout::new();
    for id in 0..7 {
        g.add(id);
    }
    let out = compute_grid_layout(area(), &g);
    assert_eq!(out.full.len(), 6);
    assert_eq!(out.minimized.len(), 1);
    // The full grid is pushed up to make room for the bottom strip.
    let max_full_bottom = out.full.iter().map(|(_, r)| r.y + r.height).max().unwrap();
    let strip_top = out.minimized[0].1.y;
    assert!(max_full_bottom <= strip_top, "full grid overlaps strip");
    // Strip sits at the bottom with the reserved height.
    assert_eq!(out.minimized[0].1.height, MINIMIZED_STRIP_HEIGHT);
    assert_eq!(out.minimized[0].1.y, 40 - MINIMIZED_STRIP_HEIGHT);
}

#[test]
fn compose_full_ids_match_layout_order() {
    let mut g = GridLayout::new();
    g.add(0);
    g.add(1);
    g.add(2);
    let out = compute_grid_layout(area(), &g);
    let ids: Vec<usize> = out.full.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![2, 1, 0]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p legacy-core grid::`
Expected: FAIL — `compute_grid_layout` / `GridRects` not found.

- [ ] **Step 3: Implement `compute_grid_layout`**

Create `crates/legacy-core/src/grid/compose.rs`:

```rust
use ratatui::layout::Rect;

use super::geometry::grid_rects;
use super::state::GridLayout;

/// Height (rows) reserved for the minimized thumbnail strip when any tile is
/// minimized.
pub const MINIMIZED_STRIP_HEIGHT: u16 = 3;

/// Concrete placement of every tile for one frame.
#[derive(Debug, Clone, Default)]
pub struct GridRects {
    /// Full-size tiles: `(tile_id, rect)` in most-recently-active order.
    pub full: Vec<(usize, Rect)>,
    /// Minimized thumbnails: `(tile_id, rect)` left-to-right.
    pub minimized: Vec<(usize, Rect)>,
}

/// Place a `GridLayout` into `area`: grid the full tiles in the main region,
/// and — when there are minimized tiles — reserve a bottom strip and lay them
/// out evenly across it.
pub fn compute_grid_layout(area: Rect, layout: &GridLayout) -> GridRects {
    let full_ids = layout.full();
    let min_ids = layout.minimized();
    if full_ids.is_empty() && min_ids.is_empty() {
        return GridRects::default();
    }

    let (grid_area, strip_area) = if min_ids.is_empty() {
        (area, None)
    } else {
        let strip_h = MINIMIZED_STRIP_HEIGHT.min(area.height);
        let grid_h = area.height.saturating_sub(strip_h);
        let grid = Rect::new(area.x, area.y, area.width, grid_h);
        let strip = Rect::new(area.x, area.y + grid_h, area.width, strip_h);
        (grid, Some(strip))
    };

    let full: Vec<(usize, Rect)> = full_ids
        .iter()
        .copied()
        .zip(grid_rects(grid_area, full_ids.len()))
        .collect();

    let minimized = match strip_area {
        Some(strip) => min_ids
            .iter()
            .copied()
            .zip(grid_rects(strip, min_ids.len()))
            .collect(),
        None => Vec::new(),
    };

    GridRects { full, minimized }
}
```

Modify `crates/legacy-core/src/grid/mod.rs`:

```rust
//! Agent grid layout: pack N tiles into a near-square grid and track which
//! tiles are shown full vs. minimized (LRU). UI-independent; consumed by the
//! renderer.

mod compose;
mod geometry;
mod state;

#[cfg(test)]
mod tests;

pub use compose::{compute_grid_layout, GridRects, MINIMIZED_STRIP_HEIGHT};
pub use geometry::{grid_rects, MAX_FULL_TILES};
pub use state::GridLayout;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p legacy-core grid::`
Expected: PASS — compose tests green.

- [ ] **Step 5: Format, lint, full workspace test**

Run: `cargo fmt && cargo clippy --workspace -- -W clippy::all && cargo test --workspace`
Expected: no new clippy warnings in `grid/`; entire workspace still compiles and all existing tests pass (this module is additive — nothing else changed).

- [ ] **Step 6: Commit**

```bash
git add crates/legacy-core/src/grid/compose.rs crates/legacy-core/src/grid/mod.rs crates/legacy-core/src/grid/tests.rs
git commit -m "feat(core): compute_grid_layout — grid + minimized strip placement"
```

---

## Self-Review

- **Spec coverage:** "grow in grid" → Task 1 (`grid_rects`, near-square). "6-tile cap, minimize oldest/least-active" → Task 2 (`GridLayout` LRU, `MAX_FULL_TILES`). "minimized thumbnail strip" → Task 3 (`compute_grid_layout` reserves the strip). Empty-canvas default and explorer are out of scope here (Plan 3). Wiring into the live renderer is out of scope here (Plan 2). ✅
- **Placeholder scan:** every code step contains complete code; no TODO/TBD. ✅
- **Type consistency:** `GridLayout`, `grid_rects`, `MAX_FULL_TILES`, `GridRects`, `compute_grid_layout`, `MINIMIZED_STRIP_HEIGHT` used identically across tasks and `mod.rs` re-exports. `full()`/`minimized()`/`touch()`/`add()`/`remove()` names stable. ✅
- **File-size limit:** geometry.rs ≈55 lines, state.rs ≈60, compose.rs ≈65, tests.rs grows but is test-only (the 200-line rule is about production `.rs`; if tests.rs nears 200, split into `tests/geometry.rs` + `tests/state.rs` style — note for the implementer). ✅

---

## The wider sequence (Plans 2–4, to be written next)

This plan delivers a tested engine but no visible change. The remaining plans wire it in and complete the pivot, each leaving the app working:

- **Plan 2 — Wire the grid + prefix-key navigation.** Replace the terminal render path (`app/render/panels.rs`, `app/terminals.rs`) so agents use `GridLayout`/`compute_grid_layout` instead of `LayoutNode::split_leaf`. Call `add` on spawn, `remove` on close, `touch` on focus/output. Render minimized thumbnails (title + token/status). Introduce the **prefix key**: reserve one chord, pass all other keys (incl. `Tab`) through to the focused agent, map `prefix→arrows` (grid nav), `prefix→c/x/z` (spawn/close/maximize). Unit-test key routing.
- **Plan 3 — Hideable explorer + empty-canvas default.** Demote the file panel to a toggleable left sidebar (`prefix→e`), default hidden; empty canvas when no agents. Keep the existing editor overlay for opening files.
- **Plan 4 — Strip the file-manager subsystems.** Remove dual-panel file ops, the F-key bar, copy/move/rename/chmod/batch-rename/bookmarks and their actions/keymap/components. Demolition last, once the new model fully replaces them.
