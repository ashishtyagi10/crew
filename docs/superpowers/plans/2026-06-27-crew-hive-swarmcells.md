# crew-hive Swarm Cell Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn a `FleetView` (constellation or heatmap layout) into a concrete grid of placed glyphs — the last UI-independent, headless-testable step before the GPU draw. crew-app will map each glyph to a `CellView` and paint it; this module owns the "where does each node/cell land on a cols×rows grid, and what glyph/color" logic so it can be unit-tested without a GPU.

**Architecture:** Extend the existing `crate::view` module with a `render` submodule. `CellGlyph { col, row, ch, color }` is a plain placed glyph. `render_cells(view: &FleetView, cols: u16, rows: u16) -> Vec<CellGlyph>` maps a constellation's normalized node coords to grid cells (node → a filled dot glyph at its scaled position; clamped in-bounds; collisions resolved by last-writer) and a heatmap's row/col cells to a block glyph grid scaled to fit. Deterministic. No GPU, no GUI.

**Tech Stack:** Rust, `serde`, `cargo test`. No new deps.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total.
- **No new dependencies.**
- Output is grid-cell coordinates (`u16` col/row within `0..cols`/`0..rows`), deterministic, never out of bounds.
- `CellGlyph` derives `serde::{Serialize, Deserialize}` (the renderer/remote bridge may ship glyph grids).
- crew-hive depends on no other crew crate.
- Dead code removed, not suppressed.
- Consumes: `crate::view::{FleetView, Constellation, Heatmap, Node, Cell, Rgb}`.

---

### Task 1: render_cells (constellation + heatmap → glyph grid)

**Files:**
- Create: `crates/crew-hive/src/view/render.rs`
- Modify: `crates/crew-hive/src/view/mod.rs` (declare `mod render;` + re-export)
- Modify: `crates/crew-hive/src/view/tests.rs` (append render tests)

**Interfaces:**
- Produces (in `crate::view`):
  - `pub struct CellGlyph { pub col: u16, pub row: u16, pub ch: char, pub color: Rgb }` — `Clone, Debug, PartialEq, Serialize, Deserialize`.
  - `pub fn render_cells(view: &FleetView, cols: u16, rows: u16) -> Vec<CellGlyph>`:
    - **Constellation:** for each `Node`, `col = (node.x * (cols-1)) rounded`, `row = (node.y * (rows-1)) rounded`, both clamped to `0..cols`/`0..rows`; glyph `ch = '●'`, `color = node.color`. (Edges are not drawn as cells in v1 — the GPU layer can draw lines between node rects; keep this function nodes-only and note it.) Deterministic order = node order. Guard `cols==0 || rows==0` → empty.
    - **Heatmap:** scale the heatmap's `cols`/`rows` grid into the `cols`/`rows` viewport — for each `Cell`, `out_col = (cell.col * (cols-1) / max(view_cols-1,1))`, `out_row = (cell.row * (rows-1) / max(view_rows-1,1))` (integer scaled, clamped); glyph `ch = '■'`, `color = cell.color`. If the heatmap is small enough to fit 1:1 (`view_cols <= cols && view_rows <= rows`), place at `(cell.col, cell.row)` directly.
- Behavior (tested): a constellation node at x=0,y=0 → (0,0); at x=1,y=1 → (cols-1, rows-1); glyphs carry the node's color; a heatmap cell at (0,0) → (0,0); out-of-viewport never produced.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-hive/src/view/tests.rs`:
```rust
#[test]
fn render_constellation_places_nodes_in_bounds() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0])]).unwrap();
    let view = FleetView::Constellation(constellation(&g, &Fleet::new()));
    let cells = render_cells(&view, 40, 20);
    assert_eq!(cells.len(), 2);
    for c in &cells {
        assert!(c.col < 40 && c.row < 20);
        assert_eq!(c.ch, '●');
    }
    // the root (x=0) lands at col 0; the leaf (x=1.0) lands at the right edge
    let cols_sorted: Vec<u16> = { let mut v: Vec<u16> = cells.iter().map(|c| c.col).collect(); v.sort_unstable(); v };
    assert_eq!(cols_sorted[0], 0);
    assert_eq!(*cols_sorted.last().unwrap(), 39);
}

#[test]
fn render_heatmap_fits_small_grid_one_to_one() {
    let mut fleet = Fleet::new();
    for i in 0..4u64 {
        fleet.apply(&HiveEvent::AgentSpawned { agent: AgentId(i), task: TaskId(i) });
    }
    let view = FleetView::Heatmap(heatmap(&fleet, 2)); // 2 cols x 2 rows
    let cells = render_cells(&view, 40, 20);
    assert_eq!(cells.len(), 4);
    for c in &cells {
        assert!(c.col < 40 && c.row < 20);
        assert_eq!(c.ch, '■');
    }
}

#[test]
fn render_zero_viewport_is_empty() {
    let g = TaskGraph::new(vec![spec(0, &[])]).unwrap();
    let view = FleetView::Constellation(constellation(&g, &Fleet::new()));
    assert!(render_cells(&view, 0, 10).is_empty());
}
```

- [ ] **Step 2: Run fail → implement → pass**

Run `cargo test -p crew-hive view::` (FAIL), implement `view/render.rs` (the `CellGlyph` struct + `render_cells` per the formulas; rounding via `(v * (n-1) as f32).round() as u16` then `.min(n-1)`), declare `mod render; pub use render::{CellGlyph, render_cells};` in `view/mod.rs`, run again (PASS). Keep ≤ 200 lines.

- [ ] **Step 3: Full gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets`.
```bash
git add crates/crew-hive/src/view
git commit -m "feat(hive): render_cells — FleetView -> placed glyph grid (headless)"
```

---

## Self-Review

- **Spec coverage:** the swarm view's "render the fleet to the GPU as cells" — this is the UI-independent half (layout → glyph grid); the GPU draw of these glyphs + drill-down is the crew-app wiring that needs a GPU to verify. ✅
- **Placeholder scan:** complete interfaces + tests + exact scaling formulas. ✅
- **Determinism + bounds:** clamped, ordered, zero-viewport guard — all tested. ✅
- **No new deps / no GUI / no LLM.** ✅

## Where this sits

The final headless-testable rendering step. What remains to reach the on-screen sci-fi view is **crew-app wiring** (add crew-hive as a dependency; add a swarm pane that maps `CellGlyph`→`CellView` and draws node-link lines/zoom; run the tokio engine alongside the winit loop), which needs a GPU + a running app (and the LLM key for live agents) to verify — so it is handed to the user for hands-on testing rather than built unverified.
