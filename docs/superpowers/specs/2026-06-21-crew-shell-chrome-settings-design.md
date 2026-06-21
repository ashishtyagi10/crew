# Crew Shell Chrome & Settings — Design

**Status:** approved direction (user feedback after first run, 2026-06-21).
**Supersedes nothing; extends the merged Crew milestones.**

## Motivation

After the first interactive run of Crew, the user gave three pieces of feedback:

1. **Default font is very small.** Bump it — and make size configurable, not a constant.
2. **Add a collapsible left navigation panel** (toggled by a button in the top
   bar) showing **CPU, memory, and disk** usage as bar gauges.
3. **Add a settings screen** to change the font, size, and other configuration.

These three share two pieces of new infrastructure, so they are designed together
and built in dependency order:

- A **runtime configuration** object (`CrewConfig`) — font size lives here, not in
  a `const`. Settings edits mutate it; it persists to disk.
- A **chrome draw layer** — Crew currently draws only panes. The top bar, the left
  nav, the gauges, and the settings overlay are all "not a terminal pane". We add a
  minimal immediate-mode primitive API to `crew-render` (filled rects + positioned
  text) that `crew-app` feeds. `crew-render` stays dumb (draws primitives); all
  geometry and logic stay in `crew-app`. This preserves the hard boundary:
  **`crew-render` imports neither `crew-term` nor `crew-plugin`** and knows nothing
  about config, stats, or panes beyond `PaneScene`.

## Key decisions (defaults chosen; reversible)

- **In-app top bar, not OS title-bar widgets.** winit's OS decorations cannot host
  custom buttons portably, and Crew is a from-scratch GPU terminal (Warp/Ghostty
  lineage) — so we render our own ~30px top bar inside the window with the nav
  toggle button at its left and the "Crew" title. The OS title bar stays as-is.
- **Chrome = rects + text primitives.** No widget framework. A button is a rect plus
  a label plus a hit-rect; a gauge is a track rect, a fill rect, and a percent label.
- **Settings editing is keyboard-driven adjust, not free text entry.** Building a
  text-input widget is out of scope. The overlay is a list of typed fields; `↑/↓`
  selects a row, `←/→` (or `-`/`+`) adjusts the value, `Esc`/`Cmd+,` closes and
  saves. This is robust, small, and extendable.
- **Config persists as TOML** at `~/.config/crew/config.toml` (`dirs` + `toml`,
  both already workspace deps). Missing/garbage file → defaults; never panic.
- **`sysinfo`** is the system-stats source (new workspace dep). Sampled on a slow
  throttle (~1s) off the 16ms frame tick — CPU% needs a refresh interval anyway.

## Architecture

### `CrewConfig` (crew-app)
```
pub struct CrewConfig {
    pub font_size: f32,     // default 18.0  (was a 16.0 const)
    pub nav_width: f32,     // default 210.0
    pub show_nav: bool,     // default false (collapsed)
}
impl CrewConfig {
    pub fn line_height(&self) -> f32 { self.font_size * 1.25 }
    pub fn load() -> Self;   // ~/.config/crew/config.toml, defaults on any error
    pub fn save(&self);      // best-effort; logs on error, never panics
}
```
`font_size` is clamped to `[12, 32]`, `nav_width` to `[160, 320]` on load and edit.

### Runtime font metrics (crew-render)
- `Renderer::new(window, font_size)` and `CellGrid::new(gpu, font_size)` take the
  initial size. `CellGrid` stores `font_size`/`line_height` as **fields** (not
  consts) and uses them in both the probe and `set_scene`'s `FontParams`.
- `Renderer::set_font_size(&mut self, f32)` → `CellGrid::set_font_size` re-probes
  `cell_w` and recomputes `cell_h`. The app then recomputes the grid, relayouts,
  reflows every PTY, and redraws. The `FONT_SIZE`/`LINE_HEIGHT` consts become the
  `Default` for `CrewConfig` only.

### Chrome primitive API (crew-render)
```
pub struct UiRect { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub color: [f32; 4] }
pub struct UiText { pub text: String, pub x: f32, pub y: f32, pub color: (u8, u8, u8) }
// frame now takes panes + chrome primitives drawn on top, in order:
Renderer::frame(&mut self, panes: &[PaneScene], rects: &[UiRect], texts: &[UiText])
```
`UiRect`s go through the existing `QuadLayer` (appended after pane quads). `UiText`s
become extra glyphon buffers positioned absolutely (alongside pane buffers). The
settings overlay reuses the same primitives — it is just more rects + texts drawn
last (on top of everything).

### Layout reservation (crew-app)
- `chrome.rs` computes: `top_bar` rect (full width × ~30px at y=0), toggle-button
  hit-rect, `nav` rect (when `show_nav`), and the **content origin** `(cx, cy)` and
  size `(cw, ch)` left for panes.
- `pane_rects` is computed over `(cw, ch)` then translated by `(cx, cy)` (new
  `pane_rects_at(n, origin, size, gap)` or translate in the caller). Click→pane
  hit-testing already uses the rect list, so it keeps working once rects are offset.

### System stats (crew-app)
- `stats.rs`: `SysSampler { sys: sysinfo::System, last: Stats, … }` with
  `Stats { cpu: f32, mem: f32, disk: f32 }` (each 0..1). `maybe_refresh()` is called
  each tick but only re-samples past the throttle interval. The nav renders three
  gauges from `last`.

### Settings overlay (crew-app)
- `settings.rs`: `SettingsOverlay { open: bool, selected: usize }` over a static
  field list `[FontSize, NavWidth, ShowNav]`. `on_key` mutates the bound
  `CrewConfig` and signals "font changed" so the app applies `set_font_size`.
  `build(cfg, surface) -> (Vec<UiRect>, Vec<UiText>)` renders the modal.
- `Cmd+,` toggles it. While open, plain keys drive the overlay (not panes);
  `Esc` closes. On close, `CrewConfig::save`.

## Input additions
- **Top-bar toggle button click** → flip `show_nav`, relayout, redraw.
- **`Cmd+,`** → toggle settings overlay.
- Mouse click routing subtracts the content origin before pane hit-testing; clicks
  inside the top bar / nav do not change pane focus (except the toggle button).

## Build order (one plan each)

- **Plan A — Configurable runtime font.** `CrewConfig` (load/save/clamp), runtime
  `CellGrid`/`Renderer` font metrics + `set_font_size`, default bumped to 18.
  Delivers the visible "bigger font" fix immediately. Foundation for C.
- **Plan B — Top bar + collapsible left nav with gauges.** crew-render primitive API
  (`UiRect`/`UiText`, `frame` signature), `sysinfo` sampler, `chrome.rs` layout +
  content-rect reservation, toggle button + click, three gauges.
- **Plan C — Settings overlay.** `settings.rs` modal, `Cmd+,`, keyboard field
  adjust, live font apply via Plan A, TOML persistence.

Each plan ends green on the standard gate: compile + `clippy --workspace
--all-targets` zero warnings + `cargo test` + `timeout 6 cargo run -p crew-app`
exit 124 (non-panic). Every `.rs` ≤ 200 lines. Visual confirmation is the user's.

## Out of scope (deferred)
- Free-text settings fields / theme color editing UI (keyboard adjust only for now).
- Nav hosting anything beyond the three gauges (file tree, session list — later).
- Per-monitor DPI font rescaling; settings search; config hot-reload from disk.
