# Crew Stats & Settings Panes — Design

**Status:** approved direction (user feedback after first runs, 2026-06-21).
**REVISED** after the user clarified: *"everything should display in terminal — no
overlay. Settings can open as another tab/panel; so can the left panel. I'm happy
with the mono font."* This supersedes the earlier chrome/overlay draft of this doc.

## Motivation

First-run feedback produced three asks: bigger/configurable font, a system-stats
"left panel" (CPU/mem/disk), and a settings screen. The **revised** direction is
that the stats view and the settings screen are **ordinary panes in the grid**, not
floating chrome or a modal overlay. Crew's model is "everything is a tile," so we
extend the existing pane system rather than add a separate UI layer.

What already shipped (Plan A, on branch `crew-config-font`):
- Runtime, config-driven font with HiDPI scaling (mono font now sized well — the
  user is happy with it). `CrewConfig` persists to `~/.config/crew/config.toml`.
- Window: resizable 1200×800, `Cmd+M` maximize, `Cmd+Q`/`Ctrl+Q` quit. Release-mode
  run for snappy load.

## Key decisions

- **No overlay, no chrome layer.** The earlier `UiRect`/`UiText` primitive API and
  top-bar/left-strip chrome are **dropped**. Everything renders through the existing
  pane → `CellView` path that the terminal and chat panes already use.
- **Two new pane types**, alongside `Terminal` and `Chat`:
  - `PaneContent::Stats(StatsPane)` — a live CPU/MEM/DISK dashboard drawn as cells
    (labels + bar gauges built from block glyphs `█`/`░`, neon-green fill).
  - `PaneContent::Settings(SettingsPane)` — the config shown as editable lines,
    navigated and adjusted by keys, drawn as cells.
- **Opened like any pane, auto-tiled.** `Cmd+G` opens a Stats pane; `Cmd+,` opens a
  Settings pane. They pack into the near-square grid (`cols=ceil(sqrt(n))`) exactly
  like terminal/chat panes; `Cmd+W` closes them; click/`Cmd+[1-9]` focuses them.
- **Mono font unchanged.** No font work in this phase beyond what Settings exposes.
- **`sysinfo`** (new workspace dep) provides CPU/mem/disk, sampled on a ~1s throttle
  so the 16ms frame tick stays cheap.

## Architecture

### Pane contract (existing — both new types implement it)
- `Pane::cells()` already dispatches on `PaneContent`; add arms for `Stats`/`Settings`
  calling each pane's `cells(cols, rows) -> Vec<CellView>`.
- Key routing in `handler.rs`: non-Super keys go to the focused pane. `Terminal`
  writes to its PTY; `Chat` calls `on_key`. `Stats` ignores keys (read-only);
  `Settings` calls `on_key` and may return a change the app applies.
- `about_to_wait` drains each pane for redraw: `Terminal` reads PTY, `Chat` polls;
  add `Stats` → refresh the sampler past its throttle (returns `changed`),
  `Settings` → no async change (`false`).

### `SysSampler` (stats.rs, crew-app)
`Stats { cpu, mem, disk }` each `0.0..=1.0`. `SysSampler` wraps `sysinfo::System`
(+ disks), refreshes at most once/second, exposes `stats()`. Pure helper
`fraction(used, total) -> f32` (total 0 → 0; clamped) is unit-tested; the bar
renderer `gauge_cells(label, frac, row, cols) -> Vec<CellView>` is pure and tested.

### `StatsPane` (statspane.rs, crew-app)
Owns a `SysSampler`. `cells(cols, rows)` renders three rows (CPU/MEM/DISK), each a
label + a bar (`█` for the filled fraction of the available width, `░` for the rest)
+ a trailing `NN%`. Fill in accent green `(0,255,160)`, track in `(40,80,95)`,
bg `(8,8,16)`. `refresh()` ticks the sampler (throttled).

### `SettingsPane` (settingspane.rs, crew-app)
Holds a working `CrewConfig` copy + `selected: usize` over a static field list
`[FontSize, NavWidth, ShowNav]` (nav_width kept for forward-compat even though there
is no nav chrome; harmless and editable). `cells(cols, rows)` renders one line per
field (`> Font size      14`), highlighting the selected row in accent.
`on_key(&mut self, key) -> Option<SettingsChange>`: `↑/↓` move selection;
`←/→` or `-/+` adjust the selected field (font_size ±1 clamped [12,32], nav_width
±10 clamped [160,320], show_nav toggles); returns `Some(SettingsChange { config })`
when a value changed so the app can apply + persist. The app applies font changes
live (`renderer.set_font_size(config.font_size * scale)`), updates `self.config`,
and calls `config.save()`.

## Input additions
- `Cmd+G` → open a Stats pane. `Cmd+,` → open a Settings pane. Both reuse the
  existing `spawn`/relayout path. No reserved keys beyond Super-chords.

## Build order (one plan: `crew-stats-settings-panes`)
1. `sysinfo` dep + `SysSampler` + `fraction` (TDD).
2. `StatsPane` + `gauge_cells` + `PaneContent::Stats` + `Cmd+G` + `about_to_wait`
   refresh (TDD on the pure render).
3. `SettingsPane` + render + `on_key` adjust + `PaneContent::Settings` + `Cmd+,` +
   live font apply + `config.save()` (TDD on render + key reducer).
4. Cleanup + milestone verification.

Gate per task: compile + `clippy --workspace --all-targets` zero warnings +
`cargo test` + `timeout 6 cargo run -p crew-app` exit 124. Every `.rs` ≤ 200 lines.
Visual confirmation is the user's.

## Out of scope (deferred)
- Tabs (the user offered "tab or panel"; panes/tiles satisfy it — no tab bar now).
- Free-text config entry (keyboard adjust only); theme-color editing; config
  hot-reload from disk; graphs/history in the stats pane.
