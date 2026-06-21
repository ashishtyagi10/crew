# Crew Configurable Runtime Font Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make the font size a runtime value (default bumped 16 → 18) backed by a
persistent `CrewConfig`, with `Renderer::set_font_size` re-deriving cell metrics —
so the font is bigger now and live-adjustable later (settings overlay, Plan C).

**Architecture:** `crew-render`'s cell metrics move from module `const`s into
`CellGrid` fields seeded at construction and recomputed by `set_font_size`. A new
`CrewConfig` (crew-app) owns `font_size`/`nav_width`/`show_nav`, loads/saves TOML at
`~/.config/crew/config.toml`, and seeds the renderer. The app gains `apply_font_size`
that re-probes metrics, recomputes the grid, relayouts, reflows PTYs, and redraws.

**Tech Stack:** Rust 2021; existing crates + `serde`/`toml`/`dirs` (already workspace
deps) added to crew-app.

## Global Constraints
- **`crew-render` imports neither `crew-term` nor `crew-plugin`.**
- **Every `.rs` ≤ 200 lines (HARD).**
- **Reserved keys = Super-chords only;** non-Super keys go to the focused pane.
- `cargo clippy --workspace --all-targets` ZERO warnings; **no `#[allow]`**.
- Config load/save is best-effort: any error → defaults / no-op, **never panic**.
- `font_size` clamped to `[12.0, 32.0]`; `nav_width` clamped to `[160.0, 320.0]`.
- Gate: compile + clippy clean + `cargo test` + `timeout 6 cargo run -p crew-app` → exit 124.

---

### Task 1: crew-render runtime cell metrics (TDD)

**Files:**
- Modify `crates/crew-render/src/celltext.rs` (add pure `cell_metrics`).
- Modify `crates/crew-render/src/cellgrid.rs` (fields + `set_font_size`, `new` takes size).
- Modify `crates/crew-render/src/renderer.rs` (`new` takes size, add `set_font_size`).

**Interfaces:**
- Produces: `pub fn cell_metrics(fs: &mut FontSystem, font_size: f32) -> (f32, f32)`
  in celltext.rs — returns `(cell_w, cell_h)` where `cell_w = probe_cell_width(...)`
  and `cell_h = font_size * 1.25`. Reuses the existing probe buffer setup.
- `CellGrid::new(gpu: &Gpu, font_size: f32) -> Self` (stores `font_size`,
  `line_height`, `cell_w`, `cell_h`); `CellGrid::set_font_size(&mut self, f32)`
  re-runs `cell_metrics` and updates all four fields.
- `Renderer::new(window, font_size: f32)`; `Renderer::set_font_size(&mut self, f32)`
  forwards to `CellGrid`. `set_scene` uses the stored `font_size`/`line_height`
  (not the consts) in `FontParams`.

- [ ] **Step 1: Failing test** in celltext.rs `#[cfg(test)]`: build a `FontSystem`,
  assert `cell_metrics(&mut fs, 24.0)` yields strictly larger `cell_w` and `cell_h`
  than `cell_metrics(&mut fs, 12.0)`, and that `cell_h == 24.0 * 1.25` for the 24 case.
- [ ] **Step 2: Run → fail** (`cargo test -p crew-render`): `cell_metrics` undefined.
- [ ] **Step 3:** Extract `cell_metrics` from the probe logic currently inlined in
  `CellGrid::new`. The probe needs a temporary `Buffer` with `Wrap::None`; it does
  **not** need GPU surface size for width probing (probe a single glyph). Keep the
  `LINE_HEIGHT` multiple as `font_size * 1.25`. Make `cellgrid.rs` store
  `font_size`/`line_height` fields, call `cell_metrics` in `new(gpu, font_size)`, add
  `set_font_size`, and read the fields in `set_scene`. Add `Renderer::new(window,
  font_size)` + `set_font_size`. Keep the `FONT_SIZE`/`LINE_HEIGHT` consts only as
  the values crew-app's config default will reference (or delete if unused after
  Task 2 — leave for now). **If cellgrid.rs would exceed 200 lines, move the probe
  helper into celltext.rs (it already hosts `probe_cell_width`).**
- [ ] **Step 4: Run → pass;** `cargo test -p crew-render` green; clippy clean; both
  files ≤200.
- [ ] **Step 5: Commit** — `feat(crew-render): runtime cell metrics + set_font_size`.

---

### Task 2: `CrewConfig` load/save/clamp (TDD)

**Files:**
- Create `crates/crew-app/src/config.rs`.
- Modify `crates/crew-app/src/main.rs` or the crate root module list (`mod config;`).
- Modify `crates/crew-app/Cargo.toml` (add `serde`, `toml`, `dirs` workspace deps).

**Interfaces:**
- Produces:
```rust
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct CrewConfig { pub font_size: f32, pub nav_width: f32, pub show_nav: bool }
impl Default for CrewConfig { /* 18.0, 210.0, false */ }
impl CrewConfig {
    pub fn line_height(&self) -> f32 { self.font_size * 1.25 }
    pub fn clamped(self) -> Self;            // font_size→[12,32], nav_width→[160,320]
    pub fn from_toml_str(s: &str) -> Self;   // parse; on err → Default; then clamped
    pub fn to_toml_str(&self) -> String;
    pub fn config_path() -> Option<std::path::PathBuf>; // dirs::config_dir()/crew/config.toml
    pub fn load() -> Self;                    // read path → from_toml_str; missing → Default
    pub fn save(&self);                       // best-effort mkdir + write; never panic
}
```
- Serde defaults: annotate each field with `#[serde(default = "…")]` so a partial
  TOML file still parses (missing keys fall back to the default value).

- [ ] **Step 1: Failing tests** in config.rs `#[cfg(test)]`:
  (a) `CrewConfig::default().font_size == 18.0` and `.show_nav == false`;
  (b) `CrewConfig { font_size: 99.0, nav_width: 9.0, show_nav: true }.clamped()` →
      `font_size == 32.0`, `nav_width == 160.0`;
  (c) `from_toml_str("font_size = 25.0\n")` → `font_size == 25.0`, `nav_width ==
      210.0` (default filled), `show_nav == false`;
  (d) `from_toml_str("garbage {{{")` → equals `CrewConfig::default()`;
  (e) round-trip: `from_toml_str(&c.to_toml_str()) == c` for a clamped `c`.
- [ ] **Step 2: Run → fail** (`cargo test -p crew-app config`): module/methods absent.
- [ ] **Step 3:** Add the deps to `crew-app/Cargo.toml`
  (`serde = { workspace = true }`, `toml = { workspace = true }`,
  `dirs = { workspace = true }`), declare `mod config;` in the crate root, implement
  `CrewConfig`. `load`/`save` use `config_path()`; `save` creates parent dirs and
  ignores I/O errors (log to stderr). Derive `PartialEq` for the round-trip test.
- [ ] **Step 4: Run → pass;** `cargo test -p crew-app` green; clippy clean; config.rs ≤200.
- [ ] **Step 5: Commit** — `feat(crew-app): persistent CrewConfig (font/nav/show_nav)`.

---

### Task 3: wire config → renderer + `apply_font_size` (build-run-observe)

**Files:** Modify `crates/crew-app/src/app.rs`, `crates/crew-app/src/handler.rs`.

**Interfaces:**
- `CrewApp` gains `pub(crate) config: CrewConfig` (struct field; not via
  `#[derive(Default)]` magic — see below).
- `handler::run()` constructs the app with `config: CrewConfig::load()`.
- `resumed` calls `Renderer::new(window.clone(), self.config.font_size)`.
- New `CrewApp::apply_font_size(&mut self, size: f32)`: clamp via
  `CrewConfig::clamped`-style bounds, store on `config`, call
  `renderer.set_font_size`, recompute `current_grid`, `relayout` all panes with the
  new `cell_size`, reflow (relayout already resizes PTYs), and `redraw`.

- [ ] **Step 1:** Add `config: CrewConfig` to `CrewApp`. Because the struct uses
  `#[derive(Default)]`, either (a) implement `Default` so `config` =
  `CrewConfig::default()` and have `run()` overwrite it with `CrewConfig::load()`,
  or (b) keep derive (CrewConfig: Default already) and overwrite in `run()`. Use (b):
  in `run()`, `let mut app = CrewApp::default(); app.config = CrewConfig::load();`.
- [ ] **Step 2:** Change `Renderer::new(window.clone())` call in `resumed` to pass
  `self.config.font_size`.
- [ ] **Step 3:** Implement `apply_font_size`. It must: store clamped size on
  `config`; `renderer.set_font_size(size)`; then run the same relayout path the
  resize handler uses (`pane_rects` over current surface → `relayout(&mut panes,
  &rects, cell_w, cell_h)`); then `redraw`. (No key bound yet — Plan C calls this.
  Add `#[…]`-free; it is exercised by being called from a tiny temporary path? No —
  to avoid dead_code, **call it once** from `resumed` right after spawning the first
  pane with the current `config.font_size` as a no-op apply, which also guarantees
  the metrics path runs. This keeps it live without a key binding.)
- [ ] **Step 4: Gate** — `cargo build -p crew-app`; `cargo clippy --workspace
  --all-targets` ZERO warnings; `cargo test --workspace` green; every touched `.rs`
  ≤200; `timeout 6 cargo run -p crew-app` exit 124. Manual: bigger font visible.
- [ ] **Step 5: Commit** — `feat(crew-app): wire CrewConfig font into renderer + apply_font_size`.

---

### Task 4: cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` ZERO warnings.
- [ ] **Step 2:** `cargo test --workspace` all green.
- [ ] **Step 3:** Every `.rs` in `crates/crew-*` ≤200; remove any now-dead
  `FONT_SIZE`/`LINE_HEIGHT` const left unused (don't `#[allow]`).
- [ ] **Step 4:** Manual smoke (record in commit): default font is visibly larger;
  terminal still types; `Cmd+T`/`Cmd+J`/`Cmd+O`/`Cmd+W` still work; resize reflows.
- [ ] **Step 5:** Commit milestone — `chore: Crew configurable runtime font milestone`.

## Notes for the next phase
- Plan B consumes `config.nav_width`/`config.show_nav` and the chrome primitive API.
- Plan C calls `apply_font_size` live from the settings overlay and `config.save()` on close.
