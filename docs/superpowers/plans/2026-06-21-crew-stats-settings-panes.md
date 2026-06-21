# Crew Stats & Settings Panes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add two new pane types — a live system-stats dashboard (CPU/MEM/DISK
gauges) and an editable settings screen — rendered as **terminal cells inside
normal grid panes** (no overlay, no chrome). Opened with `Cmd+G` / `Cmd+,`.

**Architecture:** Extend `PaneContent` (`Terminal | Chat`) with `Stats(Box<StatsPane>)`
and `Settings(SettingsPane)`. Each implements `cells(cols, rows) -> Vec<CellView>`
exactly like `ChatPane` (see `crates/crew-app/src/chatlayout.rs::layout_cells` for
the pattern). They auto-tile in the grid and use the existing pane render/focus/close
paths. `sysinfo` feeds the stats pane; the settings pane edits `CrewConfig` and the
app applies font changes live + persists.

**Tech Stack:** Rust 2021; existing crates + `sysinfo` (new workspace dep).

## Global Constraints
- **`crew-render` is untouched** (everything is `CellView`, which already exists).
- **Every `.rs` ≤ 200 lines (HARD).**
- **Reserved keys = Super-chords only** (adding `Cmd+G`, `Cmd+,`); non-Super keys go
  to the focused pane.
- `cargo clippy --workspace --all-targets` ZERO warnings; **no `#[allow]`**.
- Neon palette: fill `(0,255,160)`, track `(40,80,95)`, bg `(8,8,16)`, label `(200,200,200)`.
- Gate: compile + clippy clean + `cargo test` + `timeout 6 cargo run -p crew-app` → exit 124.

---

### Task 1: `sysinfo` + `SysSampler` + `fraction` (TDD)

**Files:**
- Modify root `Cargo.toml` `[workspace.dependencies]`: add `sysinfo = "0.33"`.
- Modify `crates/crew-app/Cargo.toml`: add `sysinfo = { workspace = true }`.
- Create `crates/crew-app/src/stats.rs`; declare `mod stats;` in `crates/crew-app/src/main.rs`.

**Interfaces:**
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats { pub cpu: f32, pub mem: f32, pub disk: f32 } // each 0.0..=1.0
pub fn fraction(used: u64, total: u64) -> f32;                 // total 0 -> 0; clamp 0..=1
pub struct SysSampler { /* sysinfo::System + Disks + last: Option<Instant> + stats: Stats */ }
impl SysSampler {
    pub fn new() -> Self;            // construct, do one initial sample
    pub fn stats(&self) -> Stats;    // last sampled
    pub fn refresh(&mut self) -> bool; // re-sample only if >=1s since last; returns true iff it re-sampled
}
```
- `fraction(used, total)`: `if total == 0 { 0.0 } else { (used as f32 / total as f32).clamp(0.0, 1.0) }`.
- `SysSampler::sample` (private): refresh cpu usage + memory + disks, set
  `stats = Stats { cpu: global_cpu_usage/100, mem: fraction(used_memory, total_memory),
  disk: fraction(sum(total-available), sum(total)) }`.
- **sysinfo 0.33 API**: verify the exact method names against the compiler /
  `sysinfo` 0.33 docs (the API shifts between versions). Likely `System::new()`,
  `refresh_cpu_usage()`, `global_cpu_usage()`, `refresh_memory()`, `used_memory()`,
  `total_memory()`, `Disks::new_with_refreshed_list()`, `disk.total_space()`,
  `disk.available_space()`, `disks.refresh(true)`. Adapt to whatever 0.33 actually
  exposes; do not fight the version.

- [ ] **Step 1: Failing tests** (`stats.rs` `#[cfg(test)]`):
  (a) `fraction(0, 0) == 0.0`; (b) `fraction(50, 100) == 0.5`;
  (c) `fraction(200, 100) == 1.0`; (d) `Stats::default() == Stats{cpu:0.0,mem:0.0,disk:0.0}`;
  (e) `SysSampler::new()` does not panic and every field of `stats()` is in `0.0..=1.0`.
- [ ] **Step 2: Run → fail** (`cargo test -p crew-app stats`).
- [ ] **Step 3: Implement.** Throttle via `last: Option<Instant>` +
  `Duration::from_millis(1000)`. (No unit test asserts wall-clock timing — only
  `fraction` + range invariants are tested.)
- [ ] **Step 4: Run → pass;** clippy clean; stats.rs ≤200.
- [ ] **Step 5: Commit** — `feat(crew-app): sysinfo SysSampler + fraction`.

---

### Task 2: `StatsPane` + `Cmd+G` (TDD on the pure renderer)

**Files:**
- Create `crates/crew-app/src/statspane.rs`; declare `mod statspane;`.
- Modify `crates/crew-app/src/pane.rs` (`PaneContent::Stats`, `cells` arm).
- Modify `crates/crew-app/src/app.rs` (`spawn_stats_pane`, `"g"` chord) and
  `crates/crew-app/src/handler.rs` (`about_to_wait` + key-routing arms).

**Interfaces:**
```rust
// statspane.rs
pub struct StatsPane { sampler: crate::stats::SysSampler }
impl StatsPane {
    pub fn new() -> Self;
    pub fn refresh(&mut self) -> bool;        // delegates to sampler.refresh()
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView>;
}
pub fn render_stats(stats: crate::stats::Stats, cols: u16, rows: u16) -> Vec<CellView>;
pub fn gauge_cells(label: &str, frac: f32, row: u16, cols: u16) -> Vec<CellView>;
```
- `gauge_cells`: lay out `"<label> "` (label in `(200,200,200)`), then a bar that
  fills the columns between the label and a trailing `" NNN%"`. Filled cells use `'█'`
  in `(0,255,160)`; empty cells use `'░'` in `(40,80,95)`. `filled =
  (frac * bar_width).round() as usize`. All cells `bg=(8,8,16)`, not bold/italic.
  Clamp to `cols`; if `cols` is tiny just render what fits.
- `render_stats`: row 0 = `gauge_cells("CPU ", stats.cpu, 0, cols)`, row 1 =
  `"MEM "`, row 2 = `"DISK"`; only emit rows that fit within `rows`.
- `PaneContent::Stats(Box<StatsPane>)` — **box it** (the `sysinfo::System` is large;
  unboxed would trip `clippy::large_enum_variant`).

- [ ] **Step 1: Failing tests** (`statspane.rs` `#[cfg(test)]`):
  (a) `gauge_cells("CPU ", 0.5, 0, 40)` returns cells, all on `row == 0`, and the
  count of cells with `c == '█'` is within ±1 of the count with `c == '░'` (≈half
  filled at 50%);
  (b) `gauge_cells("CPU ", 0.0, 0, 40)` has zero `'█'` cells;
  (c) `gauge_cells("CPU ", 1.0, 0, 40)` has zero `'░'` cells;
  (d) `render_stats(Stats{cpu:0.1,mem:0.2,disk:0.3}, 40, 3)` emits cells spanning
  rows `0,1,2` (assert the set of distinct `row` values is `{0,1,2}`).
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement** `statspane.rs`. Add `PaneContent::Stats(Box<StatsPane>)`;
  in `Pane::cells` add `PaneContent::Stats(s) => s.cells(self.grid.cols, self.grid.rows)`.
  In `app.rs` add `pub(crate) fn spawn_stats_pane(&mut self)` mirroring the existing
  `spawn_new_pane`/`spawn_chat_pane` push+relayout+focus path, building a `Pane` with
  `PaneContent::Stats(Box::new(StatsPane::new()))`, `label: None`. Add `"g" =>
  self.spawn_stats_pane(),` to `handle_super_chord`.
  In `handler.rs` `about_to_wait` add arm `PaneContent::Stats(s) => s.refresh()` (the
  bool is the `changed` flag — repaints ~1/s only). In the non-Super key routing
  `match &mut pane.content`, add `PaneContent::Stats(_) => {}` (read-only).
- [ ] **Step 4: Gate** — build; clippy `--workspace --all-targets` ZERO; `cargo test
  --workspace` green; all `.rs` ≤200; `timeout 6 cargo run -p crew-app` exit 124.
  Manual: `Cmd+G` opens a stats pane with three live gauges.
- [ ] **Step 5: Commit** — `feat(crew-app): StatsPane dashboard + Cmd+G`.

---

### Task 3: `SettingsPane` + `Cmd+,` + live apply (TDD on render + key reducer)

**Files:**
- Create `crates/crew-app/src/settingspane.rs`; declare `mod settingspane;`.
- Modify `crates/crew-app/src/pane.rs` (`PaneContent::Settings`, `cells` arm).
- Modify `crates/crew-app/src/app.rs` (`spawn_settings_pane`, `apply_settings`, `","` chord).
- Modify `crates/crew-app/src/handler.rs` (key-routing arm + apply).

**Interfaces:**
```rust
// settingspane.rs
pub struct SettingsChange { pub config: crate::config::CrewConfig }
pub struct SettingsPane { cfg: crate::config::CrewConfig, selected: usize }
impl SettingsPane {
    pub fn new(cfg: crate::config::CrewConfig) -> Self;     // selected = 0
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView>;
    pub fn on_key(&mut self, key: &winit::event::KeyEvent) -> Option<SettingsChange>;
}
// pure, testable:
pub fn render_settings(cfg: &CrewConfig, selected: usize, cols: u16, rows: u16) -> Vec<CellView>;
```
- Fields list (length 3): `0 Font size`, `1 Nav width`, `2 Show nav`.
- `on_key` (pressed only): `Up` → `selected = selected.saturating_sub(1)`, return
  `None`; `Down` → `selected = (selected + 1).min(2)`, return `None`;
  `Left` or char `'-'` → adjust selected field by `-1`, return `Some(change)`;
  `Right` or char `'+'`/`'='` → adjust by `+1`, return `Some(change)`. Adjust:
  font_size `±1.0` clamp `[12,32]`; nav_width `±10.0` clamp `[160,320]`; show_nav
  toggles on any adjust. `change.config = self.cfg`.
- `render_settings`: one line per field, `"<name>   <value>"` (font/nav numeric,
  show_nav `on`/`off`); the `selected` row is prefixed `"> "` and drawn in accent
  `(0,255,160)`; others in `(200,200,200)`; bg `(8,8,16)`.
- `PaneContent::Settings(SettingsPane)` — unboxed is fine (small struct; `ChatPane`
  is already the largest unboxed variant, so this adds nothing). If clippy
  `large_enum_variant` fires, box it.

- [ ] **Step 1: Failing tests** (`settingspane.rs` `#[cfg(test)]`):
  (a) `render_settings(&CrewConfig::default(), 0, 40, 3)` emits cells on rows
  `{0,1,2}`, and row 0 begins with `'>'` (selected marker);
  (b) construct `SettingsPane::new(CrewConfig::default())`; feed a synthetic
  `KeyEvent` for `'+'` while `selected==0` → returns `Some` and the returned
  `config.font_size == 15.0` (14 default + 1);
  (c) feed `'-'` four times from default → clamps at `12.0`;
  (d) move selection `Down` twice then `'+'` → `show_nav` toggled to `true`.
  (Build the `KeyEvent`s with a small test helper; if constructing a real winit
  `KeyEvent` in a test is impractical, factor the decision into a pure
  `fn reduce_key(cfg: &mut CrewConfig, selected: &mut usize, k: KeyAction) -> bool`
  taking a local `enum KeyAction { Up, Down, Inc, Dec }`, test THAT directly, and
  have `on_key` translate the winit event into `KeyAction` then call it. Prefer this
  factoring — it keeps the logic unit-testable without winit.)
- [ ] **Step 2: Run → fail.**
- [ ] **Step 3: Implement.** Add `PaneContent::Settings(...)`; `Pane::cells` arm.
  In `app.rs`: `spawn_settings_pane(&mut self)` seeds `SettingsPane::new(self.config)`
  and pushes/relayouts/focuses; `apply_settings(&mut self, cfg: CrewConfig)` sets
  `self.config = cfg`, computes `scale = self.window.as_ref().map(|w| w.scale_factor()
  as f32).unwrap_or(1.0)`, calls `renderer.set_font_size(cfg.font_size * scale)`,
  `self.config.save()`, `redraw`. Add `"," => self.spawn_settings_pane(),` to
  `handle_super_chord`. In `handler.rs` key routing: capture
  `let mut applied: Option<CrewConfig> = None;` then in the `match` add
  `PaneContent::Settings(s) => { applied = s.on_key(&event).map(|c| c.config); }`;
  AFTER the `if let Some(pane)` block ends, `if let Some(cfg) = applied {
  self.apply_settings(cfg); }`.
- [ ] **Step 4: Gate** — build; clippy ZERO; `cargo test --workspace` green; all
  `.rs` ≤200; `timeout 6 cargo run -p crew-app` exit 124. Manual: `Cmd+,` opens a
  settings pane; `↑/↓` select; `-`/`+` adjust font size and the **font resizes
  live**; reopening shows persisted value (`~/.config/crew/config.toml` written).
- [ ] **Step 5: Commit** — `feat(crew-app): SettingsPane + Cmd+, with live apply + persist`.

---

### Task 4: cleanup + milestone verification

- [ ] **Step 1:** `cargo fmt --all`; `cargo clippy --workspace --all-targets` ZERO warnings.
- [ ] **Step 2:** `cargo test --workspace` all green.
- [ ] **Step 3:** Every `.rs` in `crates/crew-*` ≤200.
- [ ] **Step 4:** Manual smoke (record in commit): `Cmd+G` stats pane with live
  CPU/MEM/DISK gauges; `Cmd+,` settings pane adjusts font live + persists; both tile
  in the grid; `Cmd+T/J/O/W`, `Cmd+M`, `Cmd+Q`, click-focus all still work.
- [ ] **Step 5:** Commit milestone — `chore: Crew stats + settings panes milestone`.

## Notes
- No tab bar, no chrome, no overlay — panes only (per user direction 2026-06-21).
- `nav_width` stays in `CrewConfig` for forward-compat; it is editable but currently
  affects nothing visible (kept to avoid churn / for a future docked layout).
