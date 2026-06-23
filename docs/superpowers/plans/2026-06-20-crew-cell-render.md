# Crew Real Cell Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn Crew's single pane from a flat whole-grid string into a real terminal surface: per-cell foreground colors, background colors, bold/italic, and window-resize reflow — with wgpu frame orchestration encapsulated in `crew-render`.

**Architecture:** `crew-term`'s `RenderCell` grows color + style fields (resolved to concrete RGB inside the adapter, so the renderer stays palette-agnostic). `crew-render` gains a `CellGrid` renderer: a background-quad wgpu pipeline (instanced colored rects) drawn behind per-cell glyphon text carrying per-cell color/weight/style. `crew-app` stops hand-rolling wgpu — it calls a single `Renderer::frame(cells, grid)` on `crew-render` — and translates window size into terminal cols/rows on resize.

**Tech Stack:** Rust 2021, `alacritty_terminal` =0.26.0, `glyphon` =0.11.0, `wgpu` =29.0.3, `winit` =0.30.13, `portable-pty` =0.9.0.

## Global Constraints

- **The renderer (`crew-render`) must NEVER import `crew-term`.** Cells cross the boundary as `crew-render`'s own plain `CellView` value type (see Task 3); `crew-app` maps `crew_term::RenderCell` → `crew_render::CellView`.
- **`alacritty_terminal` color resolution stays inside `crew-term`** — `RenderCell` exposes concrete `(u8,u8,u8)` RGB, never alacritty `Color` enums.
- Pinned, MIT/Apache deps only. Rust 2021 stable.
- `.rs` files focused/small (split before ~200 lines). `cargo fmt` + `cargo clippy --workspace --all-targets` warning-free.
- GPU tasks cannot be visually verified here: the gate is **compile + clippy clean + `cargo test` + timeboxed non-panic launch (`timeout 6 cargo run -p crew-app` → exit 124 = pass)**. Visual confirmation is the human's, at the end.
- Reconcile any from-memory `wgpu`/`glyphon` API in this plan against the real pinned crate (read `~/.cargo/registry/src/.../<crate>-<ver>/`); the compile loop is truth.

---

### Task 1: `crew-term` — `RenderCell` carries color + style (TDD)

**Files:**
- Modify: `crates/crew-term/src/model.rs`

**Interfaces:**
- Produces (extends existing): `RenderCell { col: u16, row: u16, c: char, fg: (u8,u8,u8), bg: (u8,u8,u8), bold: bool, italic: bool }`. The existing `{col,row,c}` fields keep their names/types; new fields are added. `TermModel::cells()` populates them.

- [ ] **Step 1: Write the failing test** (append to the existing `#[cfg(test)] mod tests` in model.rs)

```rust
#[test]
fn sgr_red_bold_is_resolved_to_rgb_and_flags() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    // ESC[1m bold, ESC[31m red foreground, then "X"
    term.feed(b"\x1b[1m\x1b[31mX");
    let cell = term.cells().into_iter().find(|c| c.c == 'X').expect("cell X");
    assert!(cell.bold, "bold flag should be set");
    // Default ANSI red has a high red channel and low green/blue.
    assert!(cell.fg.0 > 120 && cell.fg.1 < 100 && cell.fg.2 < 100, "fg should be reddish, got {:?}", cell.fg);
}
```

- [ ] **Step 2: Run it, verify failure**

Run: `cargo test -p crew-term sgr_red_bold`
Expected: FAIL — `RenderCell` has no `bold`/`fg` fields.

- [ ] **Step 3: Extend `RenderCell` and resolve colors in `TermCore::cells()`**

Add the fields to `RenderCell`. In `TermCore::cells()`, for each visible `Indexed<Cell>`, map `cell.fg` / `cell.bg` (`alacritty_terminal::vte::ansi::Color`) to concrete RGB, and read `cell.flags` for bold/italic. Color resolution:
- `Color::Spec(rgb)` → `(rgb.r, rgb.g, rgb.b)`.
- `Color::Named(named)` and `Color::Indexed(i)` → look up the term's color table. `renderable_content()` exposes a `colors` palette (`&Colors`, indexable by the 256-color index; `NamedColor` casts to its index). Resolve via that table; fall back to a sensible default (e.g. light grey fg `(220,220,220)`, near-black bg `(10,10,18)`) if a slot is `None`.
- Flags: `cell.flags.contains(Flags::BOLD)` / `Flags::ITALIC`. Inverse handling (swap fg/bg) may be added here if `Flags::INVERSE` is set.

> Verify against `alacritty_terminal` 0.26.0: the exact `Color`/`NamedColor`/`Flags` paths, how `renderable_content()` surfaces the palette (`colors` field type and indexing), and the `Cell.fg`/`bg`/`flags` field names. Adjust to the real API. Keep resolution logic in a small private helper (e.g. `fn resolve(color, palette, default) -> (u8,u8,u8)`); split into a `color.rs` submodule if model.rs nears 200 lines.

- [ ] **Step 4: Run the test, verify pass**

Run: `cargo test -p crew-term` — all tests (incl. existing `feeding_text_appears_in_cells`, `echo_roundtrips_through_pty`) pass.

- [ ] **Step 5: clippy + commit**

Run: `cargo clippy -p crew-term --all-targets` (clean).
```bash
git add crates/crew-term
git commit -m "feat(crew-term): RenderCell carries resolved fg/bg RGB + bold/italic"
```

---

### Task 2: `crew-render` — background-color quad pipeline (build-run-observe)

**Files:**
- Create: `crates/crew-render/src/quads.rs`, `crates/crew-render/src/quads.wgsl`
- Modify: `crates/crew-render/src/lib.rs`

**Interfaces:**
- Produces: `pub struct QuadLayer` with `QuadLayer::new(device: &wgpu::Device, format: wgpu::TextureFormat) -> QuadLayer`, `set_quads(&mut self, device, quads: &[Quad])`, `draw(&self, &mut wgpu::RenderPass)`; and `pub struct Quad { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub color: [f32; 4] }` (pixel coords; the shader converts to clip space using a viewport-size uniform).

- [ ] **Step 1: WGSL shader** (`quads.wgsl`) — a uniform viewport size, instanced per-quad rect+color, expand a unit triangle-strip/two-triangle quad in the vertex stage to pixel rect → clip space; fragment outputs the instance color.

```wgsl
struct Vp { size: vec2<f32>, _pad: vec2<f32> };
@group(0) @binding(0) var<uniform> vp: Vp;

struct Inst { @location(0) rect: vec4<f32>, @location(1) color: vec4<f32> };
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> };

@vertex
fn vs(@builtin(vertex_index) vi: u32, inst: Inst) -> VsOut {
    // unit quad corners (two triangles)
    var corners = array<vec2<f32>, 6>(
        vec2(0.0,0.0), vec2(1.0,0.0), vec2(0.0,1.0),
        vec2(0.0,1.0), vec2(1.0,0.0), vec2(1.0,1.0));
    let c = corners[vi];
    let px = inst.rect.xy + c * inst.rect.zw;          // pixel position
    let ndc = vec2(px.x / vp.size.x * 2.0 - 1.0, 1.0 - px.y / vp.size.y * 2.0);
    var out: VsOut;
    out.pos = vec4(ndc, 0.0, 1.0);
    out.color = inst.color;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> { return in.color; }
```

- [ ] **Step 2: Implement `QuadLayer`** (`quads.rs`) — create the pipeline (one uniform bind group for viewport size; instance vertex buffer with `step_mode: Instance`, attributes for `rect: Float32x4` @loc 0, `color: Float32x4` @loc 1; draw `6` verts × N instances). `set_quads` uploads the instance buffer (recreate or write); `draw` sets pipeline+bind group+instance buffer and `draw(0..6, 0..n)`. Update the viewport uniform on resize (expose `set_viewport(&self, queue, w, h)` or fold into `draw`).

> Reconcile against wgpu 29.0.3: `RenderPipelineDescriptor`/`VertexBufferLayout`/`VertexAttribute`, `create_shader_module`, bind group layout/uniform buffer creation, and the `RenderPass` borrow rules. Iterate on `cargo check`.

- [ ] **Step 3: Wire into lib.rs** — `mod quads; pub use quads::{Quad, QuadLayer};`

- [ ] **Step 4: Gate** — `cargo build -p crew-render`, `cargo clippy -p crew-render --all-targets` clean. (No visual yet; integration is Task 4.)

- [ ] **Step 5: Commit**
```bash
git add crates/crew-render
git commit -m "feat(crew-render): instanced background-color quad pipeline"
```

---

### Task 3: `crew-render` — per-cell colored text grid (build-run-observe)

**Files:**
- Create: `crates/crew-render/src/cellgrid.rs`
- Modify: `crates/crew-render/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct CellView { pub col: u16, pub row: u16, pub c: char, pub fg: (u8,u8,u8), pub bg: (u8,u8,u8), pub bold: bool, pub italic: bool }` (crew-render's OWN cell type — keeps the crate independent of crew-term).
  - `pub struct GridMetrics { pub cell_w: f32, pub cell_h: f32, pub cols: u16, pub rows: u16 }`.
  - `pub struct CellGrid` with `new(&Gpu) -> CellGrid`, `set_cells(&mut self, &[CellView], GridMetrics)`, `prepare(&mut self, &Gpu)`, `draw(&self, &mut wgpu::RenderPass)`. Internally composes a `QuadLayer` (backgrounds) + glyphon text (foregrounds), each cell's glyph positioned at `(col*cell_w, row*cell_h)` with per-cell `Attrs` color + weight (bold) + style (italic).

- [ ] **Step 1: Implement `CellGrid`** — `set_cells`: build the `Vec<Quad>` for non-default backgrounds and feed `QuadLayer`; build the glyphon text. For per-cell text color/weight/style use a cosmic-text `Buffer` with rich `Attrs` per char span: assemble each row's string and apply `AttrsList` spans, OR (simpler, acceptable for v1 of this) one `Buffer` per row positioned at `row*cell_h`. Monospace `Family::Monospace`; per-span `.color(Color::rgb(fg))`, `.weight(Weight::BOLD)` if bold, `.style(Style::Italic)` if italic. `prepare`/`draw` mirror the existing `TextLayer` (viewport update + renderer.prepare; draw inside the pass). Draw order in the consumer: quads first, then text.

> The existing `TextLayer` (text.rs) is the reference for glyphon setup. Reconcile `Attrs`/`AttrsList`/`Color`/`Weight`/`Style` against glyphon 0.11.0's re-exported cosmic-text. Compute `GridMetrics.cell_w/cell_h` from the font metrics (monospace advance + line height) — expose a helper so Task 6 can derive cols/rows from pixels.

- [ ] **Step 2: Wire into lib.rs** — `mod cellgrid; pub use cellgrid::{CellGrid, CellView, GridMetrics};`. `TextLayer` may stay (used by nothing after Task 4) — if it becomes dead, remove it in Task 5.

- [ ] **Step 3: Gate** — build + clippy clean.

- [ ] **Step 4: Commit**
```bash
git add crates/crew-render
git commit -m "feat(crew-render): per-cell colored text grid over quad backgrounds"
```

---

### Task 4: `crew-render` — `Renderer::frame` encapsulation; `crew-app` uses it (build-run-observe)

**Files:**
- Modify: `crates/crew-render/src/gpu.rs` (or new `renderer.rs`), `crates/crew-render/src/lib.rs`, `crates/crew-app/src/app.rs`, `crates/crew-app/src/session.rs`
- Remove: dead `TextLayer` if unused; raw wgpu frame code from `crew-app`

**Interfaces:**
- Produces: a `crew-render` method that owns the whole frame: `Renderer::frame(&mut self, cells: &[CellView], metrics: GridMetrics)` (acquire texture → begin clear pass → CellGrid prepare/draw → submit → present). `Renderer` bundles `Gpu` + `CellGrid`. `crew-app` no longer references `wgpu` types.

- [ ] **Step 1: Add `Renderer`** wrapping `Gpu` + `CellGrid` with `new(Arc<Window>) -> Result<Renderer>`, `resize(w,h)`, `metrics() -> GridMetrics`, and `frame(&mut self, &[CellView], GridMetrics)`. Move the begin_render_pass/clear/draw/submit/present logic here from `crew-app/app.rs`.
- [ ] **Step 2: `crew-app`** — replace `Gpu`+`TextLayer` ownership with a single `Renderer`. Add `fn to_cellviews(cells: &[crew_term::RenderCell]) -> Vec<crew_render::CellView>` in `session.rs` (field-for-field map). On redraw: `renderer.frame(&views, metrics)`. Remove `wgpu` from `crew-app/Cargo.toml` if no longer referenced. Replace `cells_to_string` usage (keep the function only if still used by a test; otherwise remove it).
- [ ] **Step 3: Gate** — `cargo build -p crew-app`, `cargo clippy --workspace --all-targets` clean (remove any dead code: old `TextLayer`, `cells_to_string`). `timeout 6 cargo run -p crew-app` → exit 124 (non-panic) = pass.
- [ ] **Step 4: Commit**
```bash
git add crates/crew-render crates/crew-app
git commit -m "feat(crew-render): Renderer::frame owns wgpu; crew-app drops raw wgpu"
```

---

### Task 5: `crew-app` — window-size → terminal cols/rows reflow (build-run-observe)

**Files:**
- Modify: `crates/crew-app/src/app.rs`

**Interfaces:**
- Consumes: `Renderer::metrics()` (cell pixel size). Produces: on `WindowEvent::Resized`, compute `cols = (width / cell_w).floor()`, `rows = (height / cell_h).floor()` (clamp ≥ 1), and call both `renderer.resize(w,h)` and `pty.resize(GridSize{cols,rows})` so the shell reflows.

- [ ] **Step 1: Implement resize reflow** — in `Resized`: get metrics, compute cols/rows, `renderer.resize(...)`, `pty.resize(GridSize{cols,rows})`. Spawn the PTY at an initial size derived from the window's initial inner size (not hard-coded 80×24) where possible; fall back to 80×24 if metrics aren't ready.
- [ ] **Step 2: Gate** — build + clippy clean; `timeout 6 cargo run -p crew-app` exit 124. `cargo test -p crew-term` still green.
- [ ] **Step 3: Commit**
```bash
git add crates/crew-app
git commit -m "feat(crew-app): reflow terminal grid on window resize"
```

---

### Task 6: Cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all` ; `cargo clippy --workspace --all-targets` (fix any warnings, no `#[allow]`).
- [ ] **Step 2:** `cargo test -p crew-term` — all green (color + existing tests).
- [ ] **Step 3:** Manual smoke checklist (record in commit body): colored `ls --color` output renders with correct fg/bg; bold prompt is bold; resizing the window reflows the shell (run `tput cols` after resize); no panics; clean exit.
- [ ] **Step 4:** Commit milestone.
```bash
git add -A
git commit -m "chore: Crew real-cell-rendering milestone (color + style + resize)"
```

---

## Notes for the next plan (Plan 3: Multi-pane + focus)

- Multiple `PtyTerm`s; port the prior project's grid geometry (`cols = ceil(sqrt(n))`, full-tile cap, LRU demotion) into a `crew-render` layout that places each pane's `CellGrid` in a sub-rect.
- Focus model: single focus owner (orchestrator box vs a pane), click-to-focus, one reserved jump chord, passthrough.
- Damage-driven redraw via `Term::damage()` (add `TermModel::take_damage()`), instead of rebuilding all cells each frame.
- Expand `key_to_bytes` for Ctrl/Alt combos and arrow keys (currently plain keys only).
