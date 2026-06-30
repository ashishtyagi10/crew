# Crew Paper-Reader Themes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two e-ink-reader themes (`paper-light`, `paper-dark`) to crew, switchable live via a `/theme` command and a `Ctrl-Shift-L` hotkey, with the choice persisted to config.

**Architecture:** A new dependency-light `crew-theme` crate holds a `Theme` struct (every UI color, including the 16 ANSI slots and terminal default fg/bg), two `&'static` presets, and a lock-free `AtomicU8`-backed current-theme selector read via `theme()`. The three crates that own colors today (`crew-term`, `crew-render`, `crew-app`) depend on `crew-theme` and read `theme()` instead of their hardcoded constants. Pane scenes are rebuilt every frame (`build_frame` → `build_scene`), so a theme switch only needs a `redraw()` — colors are resolved live.

**Tech Stack:** Rust (workspace, edition 2021), winit + wgpu + glyphon renderer, alacritty_terminal, ratatui (for in-pane widgets), toml/serde config.

## Global Constraints

- Rust edition 2021; new crate uses `version.workspace`, `edition.workspace`, `license.workspace`.
- `crew-theme` must have **no dependencies** (pure `std`). It is imported by `crew-term`, `crew-render`, `crew-app`; it must not import any of them (no cycles).
- `theme()` is read on the winit main thread every frame — reads MUST be lock-free and non-blocking (`AtomicU8::load(Relaxed)`). No `Mutex`/`RwLock` on the read path. (Project rule: never block the winit thread.)
- **No pure black (`(0,0,0)`) or pure white (`(255,255,255)`) in any preset field.**
- In both presets, `term_bg == page_bg` (panes read as the same sheet of paper; the renderer's "skip bg quad when cell bg == default" optimization then shows the cleared page color behind text).
- Default theme when config is unset/invalid is `paper-dark`.
- Keep `suggest::COMMANDS` in sync with `run_slash_command` (existing project rule, see `dispatch.rs` header).

---

## File Structure

**New:**
- `crates/crew-theme/Cargo.toml` — the new crate manifest.
- `crates/crew-theme/src/lib.rs` — `Theme`, `ThemeId`, `PAPER_LIGHT`, `PAPER_DARK`, global selector, `theme()`.

**Modified:**
- `Cargo.toml` (workspace) — add `crates/crew-theme` member.
- `crates/crew-term/Cargo.toml`, `crates/crew-term/src/color.rs` — read theme for default fg/bg + ANSI16.
- `crates/crew-render/Cargo.toml`, `crates/crew-render/src/{cellgrid,scene,renderer}.rs` — read theme for default bg, borders, clear color.
- `crates/crew-app/Cargo.toml` — add `crew-theme` dep.
- `crates/crew-app/src/{tui,chatlayout,inputbar,panecard,findhl}.rs` — chrome colors from theme.
- `crates/crew-app/src/palette.rs` / `config.rs` — accent default becomes theme-driven; new `theme` config field + `theme_id()`.
- `crates/crew-app/src/{dispatch,suggest,spawn}.rs` + new method — `/theme` command, palette listing, live apply.
- `crates/crew-app/src/keys.rs` + `toggles.rs` — `Ctrl-Shift-L` toggle.
- `crates/crew-app/src/handler.rs` — apply theme at startup before accent.

---

## Task 1: Create the `crew-theme` crate

**Files:**
- Create: `crates/crew-theme/Cargo.toml`
- Create: `crates/crew-theme/src/lib.rs`
- Modify: `Cargo.toml` (workspace `members`)

**Interfaces:**
- Produces:
  - `struct Theme` with public fields (all `(u8, u8, u8)` except `ansi: [(u8, u8, u8); 16]`):
    `page_bg, ink, text_muted, term_fg, term_bg, border_normal, border_focused, legend_off, accent_default, status_fg, broadcast, activity, bell, dim, placeholder, hint_fg, find_hl_bg, ansi`.
  - `enum ThemeId { PaperDark, PaperLight }` with `pub fn as_str(self) -> &'static str`, `pub fn from_str(s: &str) -> Option<ThemeId>`, `pub fn theme(self) -> &'static Theme`.
  - `static PAPER_DARK: Theme`, `static PAPER_LIGHT: Theme`.
  - `pub fn set_theme(id: ThemeId)`, `pub fn current_id() -> ThemeId`, `pub fn theme() -> &'static Theme`.

- [ ] **Step 1: Add the crate to the workspace**

Modify `Cargo.toml` (workspace) `members` to include the new crate (add the line, keep the rest):

```toml
members = [
    "crates/crew-theme",
    "crates/crew-hive",
    "crates/crew-term",
    "crates/crew-render",
    "crates/crew-app",
    "crates/crew-plugin",
]
```

- [ ] **Step 2: Write the crate manifest**

Create `crates/crew-theme/Cargo.toml`:

```toml
[package]
name = "crew-theme"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
```

- [ ] **Step 3: Write the failing tests + full implementation**

Create `crates/crew-theme/src/lib.rs`:

```rust
//! Crew's color themes. A single `Theme` struct holds every UI colour; two
//! `&'static` presets (`PAPER_DARK`, `PAPER_LIGHT`) give crew an e-ink-reader
//! look. The active theme lives behind a lock-free `AtomicU8` so the winit
//! render thread can read it every frame without blocking. No dependencies and
//! no knowledge of the other crates — they import this one.
use std::sync::atomic::{AtomicU8, Ordering};

/// Every colour the UI draws with. RGB triples; `ansi` is the 16-slot terminal
/// palette (indices 0–15) used for shell output.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// Window/pane background — also the wgpu clear colour and the terminal's
    /// default background, so cells at the default bg show the page through.
    pub page_bg: (u8, u8, u8),
    /// Primary chrome text ("ink").
    pub ink: (u8, u8, u8),
    /// Secondary/body text (slightly softer than `ink`).
    pub text_muted: (u8, u8, u8),
    /// Terminal default foreground / background for unstyled shell output.
    pub term_fg: (u8, u8, u8),
    pub term_bg: (u8, u8, u8),
    /// Unfocused / focused rounded pane border.
    pub border_normal: (u8, u8, u8),
    pub border_focused: (u8, u8, u8),
    /// Legend text on an unfocused pane card.
    pub legend_off: (u8, u8, u8),
    /// Default accent when the user hasn't set one in config.
    pub accent_default: (u8, u8, u8),
    /// Status line / scroll hint amber.
    pub status_fg: (u8, u8, u8),
    /// Broadcast indicator.
    pub broadcast: (u8, u8, u8),
    /// Pane activity dot.
    pub activity: (u8, u8, u8),
    /// Bell indicator.
    pub bell: (u8, u8, u8),
    /// Dim hint text on the input bar.
    pub dim: (u8, u8, u8),
    /// Input placeholder text.
    pub placeholder: (u8, u8, u8),
    /// Hint text (chat layout).
    pub hint_fg: (u8, u8, u8),
    /// Search-highlight background.
    pub find_hl_bg: (u8, u8, u8),
    /// 16-colour ANSI palette for shell output (muted "ink" tones).
    pub ansi: [(u8, u8, u8); 16],
}

/// Warm e-ink "night" page — charcoal-brown, never blue-black. The default.
pub static PAPER_DARK: Theme = Theme {
    page_bg: (32, 32, 28),
    ink: (207, 199, 184),
    text_muted: (170, 162, 148),
    term_fg: (207, 199, 184),
    term_bg: (32, 32, 28),
    border_normal: (74, 70, 61),
    border_focused: (138, 132, 116),
    legend_off: (107, 101, 87),
    accent_default: (199, 154, 94),
    status_fg: (204, 170, 106),
    broadcast: (181, 138, 168),
    activity: (125, 154, 184),
    bell: (204, 170, 106),
    dim: (110, 104, 92),
    placeholder: (95, 90, 80),
    hint_fg: (107, 101, 87),
    find_hl_bg: (74, 67, 31),
    ansi: [
        (50, 49, 44),    // 0  black
        (192, 106, 90),  // 1  red
        (154, 167, 106), // 2  green
        (204, 170, 106), // 3  yellow
        (125, 154, 184), // 4  blue
        (181, 138, 168), // 5  magenta
        (127, 176, 170), // 6  cyan
        (207, 199, 184), // 7  white
        (107, 101, 87),  // 8  bright black
        (214, 128, 112), // 9  bright red
        (176, 189, 128), // 10 bright green
        (224, 192, 128), // 11 bright yellow
        (147, 176, 206), // 12 bright blue
        (203, 160, 190), // 13 bright magenta
        (149, 198, 192), // 14 bright cyan
        (236, 229, 214), // 15 bright white
    ],
};

/// Warm paper "day" page — soft off-white with ink-toned output.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (244, 241, 234),
    ink: (43, 40, 37),
    text_muted: (90, 84, 75),
    term_fg: (43, 40, 37),
    term_bg: (244, 241, 234),
    border_normal: (201, 194, 178),
    border_focused: (140, 132, 117),
    legend_off: (168, 159, 141),
    accent_default: (156, 107, 63),
    status_fg: (150, 110, 40),
    broadcast: (150, 70, 120),
    activity: (60, 100, 140),
    bell: (160, 120, 40),
    dim: (140, 132, 118),
    placeholder: (160, 152, 138),
    hint_fg: (160, 152, 138),
    find_hl_bg: (232, 220, 168),
    ansi: [
        (43, 40, 37),    // 0  black
        (156, 59, 46),   // 1  red (brick)
        (93, 107, 58),   // 2  green (sage)
        (154, 123, 46),  // 3  yellow (ochre)
        (63, 90, 120),   // 4  blue (faded indigo)
        (125, 75, 110),  // 5  magenta (mauve)
        (63, 111, 107),  // 6  cyan (teal)
        (92, 86, 75),    // 7  white (warm gray)
        (120, 113, 99),  // 8  bright black
        (178, 82, 66),   // 9  bright red
        (122, 134, 82),  // 10 bright green
        (180, 148, 74),  // 11 bright yellow
        (88, 116, 148),  // 12 bright blue
        (150, 100, 135), // 13 bright magenta
        (88, 140, 134),  // 14 bright cyan
        (60, 56, 50),    // 15 bright white (boldest ink)
    ],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    PaperDark,
    PaperLight,
}

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "paper-dark",
            ThemeId::PaperLight => "paper-light",
        }
    }

    pub fn from_str(s: &str) -> Option<ThemeId> {
        match s.trim() {
            "paper-dark" => Some(ThemeId::PaperDark),
            "paper-light" => Some(ThemeId::PaperLight),
            _ => None,
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::PaperDark => &PAPER_DARK,
            ThemeId::PaperLight => &PAPER_LIGHT,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            ThemeId::PaperDark => 0,
            ThemeId::PaperLight => 1,
        }
    }

    fn from_u8(v: u8) -> ThemeId {
        match v {
            1 => ThemeId::PaperLight,
            _ => ThemeId::PaperDark,
        }
    }
}

/// Active theme id, default `PaperDark` (0). Lock-free for per-frame reads.
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Set the active theme (startup, `/theme`, hotkey).
pub fn set_theme(id: ThemeId) {
    CURRENT.store(id.as_u8(), Ordering::Relaxed);
}

/// The active theme id.
pub fn current_id() -> ThemeId {
    ThemeId::from_u8(CURRENT.load(Ordering::Relaxed))
}

/// The active theme. Read every frame on the winit thread — lock-free.
pub fn theme() -> &'static Theme {
    current_id().theme()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises tests that mutate the process-wide CURRENT.
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn default_is_paper_dark() {
        let _g = guard();
        // At rest (no set_theme yet in this process) the default id is PaperDark.
        // We don't assert on a possibly-mutated global; just the mapping.
        assert_eq!(ThemeId::from_u8(0), ThemeId::PaperDark);
    }

    #[test]
    fn id_string_round_trip() {
        for id in [ThemeId::PaperDark, ThemeId::PaperLight] {
            assert_eq!(ThemeId::from_str(id.as_str()), Some(id));
        }
        assert_eq!(ThemeId::from_str("nope"), None);
        assert_eq!(ThemeId::from_str("  paper-light "), Some(ThemeId::PaperLight));
    }

    #[test]
    fn set_then_current_round_trips() {
        let _g = guard();
        set_theme(ThemeId::PaperLight);
        assert_eq!(current_id(), ThemeId::PaperLight);
        assert_eq!(theme().page_bg, PAPER_LIGHT.page_bg);
        set_theme(ThemeId::PaperDark);
        assert_eq!(current_id(), ThemeId::PaperDark);
    }

    #[test]
    fn no_preset_uses_pure_black_or_white() {
        for t in [&PAPER_DARK, &PAPER_LIGHT] {
            let mut all = vec![
                t.page_bg, t.ink, t.text_muted, t.term_fg, t.term_bg,
                t.border_normal, t.border_focused, t.legend_off, t.accent_default,
                t.status_fg, t.broadcast, t.activity, t.bell, t.dim, t.placeholder,
                t.hint_fg, t.find_hl_bg,
            ];
            all.extend_from_slice(&t.ansi);
            for c in all {
                assert_ne!(c, (0, 0, 0), "pure black found in a preset");
                assert_ne!(c, (255, 255, 255), "pure white found in a preset");
            }
        }
    }

    #[test]
    fn term_bg_equals_page_bg() {
        for t in [&PAPER_DARK, &PAPER_LIGHT] {
            assert_eq!(t.term_bg, t.page_bg);
        }
    }

    #[test]
    fn term_fg_bg_have_contrast() {
        // crude luminance gap so default text is never near-invisible.
        for t in [&PAPER_DARK, &PAPER_LIGHT] {
            let lum = |c: (u8, u8, u8)| c.0 as i32 + c.1 as i32 + c.2 as i32;
            assert!((lum(t.term_fg) - lum(t.term_bg)).abs() > 200);
        }
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-theme`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/crew-theme
git commit -m "feat(crew-theme): new crate with paper-light/paper-dark presets"
```

---

## Task 2: Drive `crew-term` color resolution from the theme

**Files:**
- Modify: `crates/crew-term/Cargo.toml`
- Modify: `crates/crew-term/src/color.rs:1-54`

**Interfaces:**
- Consumes: `crew_theme::theme()` → `term_fg`, `term_bg`, `ansi`.
- Produces: `resolve_color` now falls back to the active theme's ANSI palette and default; callers pass the theme's `term_fg`/`term_bg` as `default`.

- [ ] **Step 1: Add the dependency**

Modify `crates/crew-term/Cargo.toml` `[dependencies]` (add the line):

```toml
crew-theme = { path = "../crew-theme" }
```

- [ ] **Step 2: Write the failing test**

Append to `crates/crew-term/src/color.rs` (inside a new `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::term::color::Colors;
    use alacritty_terminal::vte::ansi::{Color, NamedColor};

    #[test]
    fn named_red_resolves_to_active_theme_ansi() {
        crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
        let palette = Colors::default(); // all slots unset → fall back to theme
        let got = resolve_color(
            Color::Named(NamedColor::Red),
            &palette,
            crew_theme::theme().term_fg,
        );
        assert_eq!(got, crew_theme::PAPER_LIGHT.ansi[1]);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p crew-term named_red_resolves_to_active_theme_ansi`
Expected: FAIL — `resolve_color` still returns the old hardcoded `ANSI16[1] = (170, 0, 0)`.

- [ ] **Step 4: Implement — read the theme**

Edit `crates/crew-term/src/color.rs`. Delete the `DEFAULT_FG`, `DEFAULT_BG`, and `ANSI16` constants (lines 4–28) and replace the two `ANSI16[idx]` fallbacks. The new file head + function:

```rust
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, Rgb};

/// The active theme's terminal default foreground.
pub(crate) fn default_fg() -> (u8, u8, u8) {
    crew_theme::theme().term_fg
}

/// The active theme's terminal default background.
pub(crate) fn default_bg() -> (u8, u8, u8) {
    crew_theme::theme().term_bg
}

pub(crate) fn resolve_color(color: Color, palette: &Colors, default: (u8, u8, u8)) -> (u8, u8, u8) {
    let ansi = &crew_theme::theme().ansi;
    match color {
        Color::Spec(Rgb { r, g, b }) => (r, g, b),
        Color::Named(named) => {
            let idx = named as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ansi[idx]
            } else {
                default
            }
        }
        Color::Indexed(i) => {
            let idx = i as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ansi[idx]
            } else {
                default
            }
        }
    }
}
```

- [ ] **Step 5: Fix the call sites in `model.rs`**

The only external users are in `crates/crew-term/src/model.rs`. Change the import on line 24 from `use crate::color::{resolve_color, DEFAULT_BG, DEFAULT_FG};` to:

```rust
use crate::color::{default_bg, default_fg, resolve_color};
```

Then update the three uses (lines ~133, ~134, ~145):

```rust
let fg = resolve_color(ind.fg, palette, default_fg());
let mut bg = resolve_color(ind.bg, palette, default_bg());
```

and the later `bg = DEFAULT_BG;` → `bg = default_bg();`.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-term`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-term
git commit -m "feat(crew-term): resolve terminal colours from the active theme"
```

---

## Task 3: Drive `crew-render` borders, default bg, and clear colour from the theme

**Files:**
- Modify: `crates/crew-render/Cargo.toml`
- Modify: `crates/crew-render/src/cellgrid.rs:11`
- Modify: `crates/crew-render/src/scene.rs:4,31-34,76` and the border-color selection
- Modify: `crates/crew-render/src/renderer.rs:84-89`

**Interfaces:**
- Consumes: `crew_theme::theme()` → `page_bg`, `border_normal`, `border_focused`.
- Produces: the renderer clears to `page_bg`; pane borders and the "default bg" sentinel come from the active theme.

- [ ] **Step 1: Add the dependency**

Modify `crates/crew-render/Cargo.toml` `[dependencies]` (add the line):

```toml
crew-theme = { path = "../crew-theme" }
```

- [ ] **Step 2: Replace the `DEFAULT_BG` constant in `cellgrid.rs`**

In `crates/crew-render/src/cellgrid.rs`, delete line 11:

```rust
pub(crate) const DEFAULT_BG: (u8, u8, u8) = (0, 0, 0);
```

and add a helper in its place:

```rust
/// The active theme's default background (the page colour). Cells at this bg
/// skip their bg quad and let the cleared page show through.
pub(crate) fn default_bg() -> (u8, u8, u8) {
    crew_theme::theme().page_bg
}
```

- [ ] **Step 3: Update `scene.rs`**

In `crates/crew-render/src/scene.rs`:

- Change the import on line 4 from `use crate::cellgrid::{CellView, DEFAULT_BG};` to:

```rust
use crate::cellgrid::{default_bg, CellView};
```

- Delete the `BORDER_NORMAL` and `BORDER_FOCUSED` constants (lines 31–34). Keep `BORDER_RADIUS` and `BORDER_THICKNESS`.
- Change the default-bg comparison (line ~76) from `if cell.bg != DEFAULT_BG {` to:

```rust
if cell.bg != default_bg() {
```

- Change the overlay backdrop quad colour (the `if pane.overlay { quads.push(Quad { ... color: [0.0, 0.0, 0.0, 1.0] }) }` block) to the page colour:

```rust
let bg = crew_theme::theme().page_bg;
quads.push(Quad {
    x: pane.x,
    y: pane.y,
    w: pane.w,
    h: pane.h,
    color: [bg.0 as f32 / 255.0, bg.1 as f32 / 255.0, bg.2 as f32 / 255.0, 1.0],
});
```

- Change the border-colour selection to read the theme:

```rust
if pane.bordered {
    let t = crew_theme::theme();
    let (r, g, b) = if pane.focused { t.border_focused } else { t.border_normal };
    let color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
    borders.push(Border {
        x: pane.x,
        y: pane.y,
        w: pane.w,
        h: pane.h,
        radius: BORDER_RADIUS,
        thickness: BORDER_THICKNESS,
        color,
    });
}
```

- [ ] **Step 4: Update the clear colour in `renderer.rs`**

In `crates/crew-render/src/renderer.rs`, replace the hardcoded clear colour (lines 84–89) with the page colour. Just above the `let mut encoder`/render-pass creation in `frame`, add `let bg = crew_theme::theme().page_bg;` then change the `Clear` to:

```rust
load: wgpu::LoadOp::Clear(wgpu::Color {
    r: bg.0 as f64 / 255.0,
    g: bg.1 as f64 / 255.0,
    b: bg.2 as f64 / 255.0,
    a: 1.0,
}),
```

(Place `let bg = ...;` before the `color_attachments` array so it's in scope.)

- [ ] **Step 5: Update `scene_tests.rs` border expectations**

Run: `grep -n "110\|210\|BORDER" crates/crew-render/src/scene_tests.rs`
The focus test (`build_scene` focused vs normal border) asserts on the old border colours. Update those assertions to the active theme:

```rust
let t = crew_theme::theme();
let f = |c: (u8, u8, u8)| [c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, 1.0];
assert_eq!(focused[0].color, f(t.border_focused));
assert_eq!(normal[0].color, f(t.border_normal));
```

(If the test instead asserts focused != normal, leave it — it still holds.)

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-render`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-render
git commit -m "feat(crew-render): clear colour, borders, default bg from the active theme"
```

---

## Task 4: Drive `crew-app` chrome colours + accent default from the theme

**Files:**
- Modify: `crates/crew-app/Cargo.toml`
- Modify: `crates/crew-app/src/tui.rs:8-9`
- Modify: `crates/crew-app/src/chatlayout.rs:6-11`
- Modify: `crates/crew-app/src/inputbar.rs:11-22`
- Modify: `crates/crew-app/src/panecard.rs:11-18`
- Modify: `crates/crew-app/src/findhl.rs:9`
- Modify: `crates/crew-app/src/config.rs:99-105` (accent fallback) and its test

**Interfaces:**
- Consumes: `crew_theme::theme()` for all chrome colours.
- Produces: `CrewConfig::accent_rgb()` falls back to `crew_theme::theme().accent_default` (not the hardcoded green) when the user hasn't set `accent`.

- [ ] **Step 1: Add the dependency**

Modify `crates/crew-app/Cargo.toml` `[dependencies]` (add the line):

```toml
crew-theme = { path = "../crew-theme" }
```

- [ ] **Step 2: Replace the chrome constants with theme reads**

For each file/constant below, delete the `const` definition and replace every use of that constant with the mapped `crew_theme::theme().<field>` expression. (`boxdraw.rs` takes colours as parameters — do **not** change it; its callers now pass theme colours.)

| File | Constant (old value) | Replace uses with |
|---|---|---|
| `tui.rs` | `DEFAULT_FG (220,220,220)` | `crew_theme::theme().ink` |
| `tui.rs` | `DEFAULT_BG (0,0,0)` | `crew_theme::theme().page_bg` |
| `chatlayout.rs` | `DEFAULT_BG (0,0,0)` | `crew_theme::theme().page_bg` |
| `chatlayout.rs` | `ACCENT_FG (0,255,160)` | `crate::palette::accent()` |
| `chatlayout.rs` | `TEXT_FG (200,200,200)` | `crew_theme::theme().text_muted` |
| `chatlayout.rs` | `INPUT_FG (220,220,220)` | `crew_theme::theme().ink` |
| `chatlayout.rs` | `HINT_FG (110,110,120)` | `crew_theme::theme().hint_fg` |
| `inputbar.rs` | `BG (0,0,0)` | `crew_theme::theme().page_bg` |
| `inputbar.rs` | `DIM (120,130,140)` | `crew_theme::theme().dim` |
| `inputbar.rs` | `TEXT_FG (220,220,220)` | `crew_theme::theme().ink` |
| `inputbar.rs` | `BROADCAST (220,120,200)` | `crew_theme::theme().broadcast` |
| `inputbar.rs` | `BORDER_ON (210,210,220)` | `crew_theme::theme().border_focused` |
| `inputbar.rs` | `BORDER_OFF (110,110,120)` | `crew_theme::theme().border_normal` |
| `inputbar.rs` | `STATUS_FG (230,180,90)` | `crew_theme::theme().status_fg` |
| `inputbar.rs` | `PLACEHOLDER (90,95,105)` | `crew_theme::theme().placeholder` |
| `panecard.rs` | `SCROLL_HINT (230,180,90)` | `crew_theme::theme().status_fg` |
| `panecard.rs` | `ACTIVITY (120,200,255)` | `crew_theme::theme().activity` |
| `panecard.rs` | `BELL (240,210,90)` | `crew_theme::theme().bell` |
| `panecard.rs` | `BROADCAST (220,120,200)` | `crew_theme::theme().broadcast` |
| `panecard.rs` | `BORDER_ON (210,210,220)` | `crew_theme::theme().border_focused` |
| `panecard.rs` | `BORDER_OFF (110,110,120)` | `crew_theme::theme().border_normal` |
| `panecard.rs` | `LEGEND_OFF (140,140,150)` | `crew_theme::theme().legend_off` |
| `panecard.rs` | `CANVAS_BG (0,0,0)` | `crew_theme::theme().page_bg` |
| `findhl.rs` | `HL_BG (90,70,0)` | `crew_theme::theme().find_hl_bg` |

Notes:
- `panecard.rs` `SCROLL_HINT`/`ACTIVITY`/`BELL`/`BROADCAST` are `pub(crate)`. If other modules import them, replace those imports with the `theme()` reads at the use sites too (`grep -rn "panecard::\(SCROLL_HINT\|ACTIVITY\|BELL\|BROADCAST\)" crates/crew-app/src`).
- `chatlayout::DEFAULT_BG`, `ACCENT_FG`, etc. are `pub`. Do the same import check: `grep -rn "chatlayout::\(DEFAULT_BG\|ACCENT_FG\|TEXT_FG\|INPUT_FG\|HINT_FG\)" crates/crew-app/src`.

- [ ] **Step 3: Make the accent default theme-driven**

In `crates/crew-app/src/config.rs`, change `accent_rgb` (lines 100–105):

```rust
/// The configured accent colour, or the active theme's default when unset/invalid.
pub fn accent_rgb(&self) -> (u8, u8, u8) {
    self.accent
        .as_deref()
        .and_then(crate::palette::parse_hex)
        .unwrap_or_else(|| crew_theme::theme().accent_default)
}
```

- [ ] **Step 4: Update the `accent_rgb` test**

In `crates/crew-app/src/config.rs` tests, replace `accent_rgb_parses_or_falls_back`:

```rust
#[test]
fn accent_rgb_parses_or_falls_back() {
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    // Unset → active theme default.
    assert_eq!(
        CrewConfig::default().accent_rgb(),
        crew_theme::PAPER_DARK.accent_default
    );
    // Valid hex → parsed.
    let cfg = CrewConfig::from_toml_str("accent = \"#102030\"\n");
    assert_eq!(cfg.accent_rgb(), (0x10, 0x20, 0x30));
    // Invalid hex → theme default (not a panic).
    let bad = CrewConfig::from_toml_str("accent = \"not-a-color\"\n");
    assert_eq!(bad.accent_rgb(), crew_theme::PAPER_DARK.accent_default);
}
```

- [ ] **Step 5: Build and test**

Run: `cargo test -p crew-app`
Expected: PASS. (Fix any leftover references to deleted constants the compiler flags.)

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app
git commit -m "feat(crew-app): chrome colours and accent default from the active theme"
```

---

## Task 5: Add the `theme` config field and apply it at startup

**Files:**
- Modify: `crates/crew-app/src/config.rs` (struct field, `Default`, `clamped`, `theme_id()`, tests)
- Modify: `crates/crew-app/src/handler.rs:~61` (apply theme before accent)

**Interfaces:**
- Consumes: `crew_theme::ThemeId`.
- Produces: `CrewConfig.theme: Option<String>` and `CrewConfig::theme_id(&self) -> crew_theme::ThemeId` (defaults to `PaperDark`).

- [ ] **Step 1: Write the failing test**

Add to `crates/crew-app/src/config.rs` tests:

```rust
#[test]
fn theme_id_parses_or_defaults() {
    assert_eq!(CrewConfig::default().theme_id(), crew_theme::ThemeId::PaperDark);
    let light = CrewConfig::from_toml_str("theme = \"paper-light\"\n");
    assert_eq!(light.theme_id(), crew_theme::ThemeId::PaperLight);
    let bad = CrewConfig::from_toml_str("theme = \"chartreuse\"\n");
    assert_eq!(bad.theme_id(), crew_theme::ThemeId::PaperDark);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-app theme_id_parses_or_defaults`
Expected: FAIL — no `theme` field / `theme_id` method.

- [ ] **Step 3: Add the field**

In `crates/crew-app/src/config.rs`, add to the `CrewConfig` struct (after `accent`):

```rust
    /// Theme name: `paper-dark` (default) or `paper-light`. Unknown/unset →
    /// `paper-dark`. Applied app-wide via [`crew_theme`].
    #[serde(default)]
    pub theme: Option<String>,
```

Add `theme: None,` to the `Default` impl, and `theme: self.theme.filter(|s| !s.is_empty()),` to `clamped`.

Add the method (next to `accent_rgb`):

```rust
/// The configured theme, or `paper-dark` when unset/unknown.
pub fn theme_id(&self) -> crew_theme::ThemeId {
    self.theme
        .as_deref()
        .and_then(crew_theme::ThemeId::from_str)
        .unwrap_or(crew_theme::ThemeId::PaperDark)
}
```

Also add `theme: None,` to any test that constructs `CrewConfig { ... }` literally without `..CrewConfig::default()` (the `round_trip` and `clamped_out_of_range` tests). For `round_trip`, set `theme: Some("paper-light".to_string()),` so the round-trip exercises it.

- [ ] **Step 4: Apply the theme at startup before the accent**

In `crates/crew-app/src/handler.rs`, find the startup line `crate::palette::set_accent(config.accent_rgb());` (~line 61) and add **above** it:

```rust
crew_theme::set_theme(config.theme_id());
```

(Order matters: accent default reads the active theme.)

- [ ] **Step 5: Run the tests**

Run: `cargo test -p crew-app`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app
git commit -m "feat(crew-app): theme config field applied at startup"
```

---

## Task 6: `/theme` command — live switch + persistence

**Files:**
- Modify: `crates/crew-app/src/spawn.rs` (new `set_theme_cmd`; apply theme in `apply_config`)
- Modify: `crates/crew-app/src/dispatch.rs:~46` and the `other` arm
- Modify: `crates/crew-app/src/suggest.rs:~99` (palette listing)

**Interfaces:**
- Consumes: `crew_theme::{ThemeId, set_theme}`, `CrewConfig::theme_id`.
- Produces: `CrewApp::set_theme_cmd(&mut self, arg: &str)` — applies a theme by name, re-applies the accent default, persists, repaints, and sets a status line.

- [ ] **Step 1: Apply the theme inside `apply_config`**

In `crates/crew-app/src/spawn.rs`, `apply_config`, add the theme apply **before** the existing `set_accent` call:

```rust
crew_theme::set_theme(self.config.theme_id());
// Apply the themeable accent app-wide (render code reads it via palette).
crate::palette::set_accent(self.config.accent_rgb());
```

- [ ] **Step 2: Write the failing test**

Add to `crates/crew-app/src/spawn.rs` (or a `#[cfg(test)] mod` near it; mirror the style of `reload.rs` tests which build `CrewApp::default()`):

```rust
#[cfg(test)]
mod theme_cmd_tests {
    use crate::app::CrewApp;

    #[test]
    fn set_theme_cmd_switches_active_theme() {
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        let mut app = CrewApp::default();
        app.set_theme_cmd("paper-light");
        assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
        assert_eq!(app.config.theme.as_deref(), Some("paper-light"));
        // Unknown name leaves the active theme unchanged.
        app.set_theme_cmd("chartreuse");
        assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    }
}
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p crew-app set_theme_cmd_switches_active_theme`
Expected: FAIL — no `set_theme_cmd`.

- [ ] **Step 4: Implement `set_theme_cmd`**

Add to `crates/crew-app/src/spawn.rs` (inside `impl CrewApp`):

```rust
/// `/theme [paper-light|paper-dark]`: switch the active theme live, persist
/// the choice, and repaint. With no/unknown arg, report the current theme.
pub(crate) fn set_theme_cmd(&mut self, arg: &str) {
    let arg = arg.trim();
    if arg.is_empty() {
        self.set_status(format!("theme: {}", crew_theme::current_id().as_str()));
        return;
    }
    let Some(id) = crew_theme::ThemeId::from_str(arg) else {
        self.set_status(format!("unknown theme '{arg}' (paper-light | paper-dark)"));
        return;
    };
    self.config.theme = Some(id.as_str().to_string());
    crew_theme::set_theme(id);
    // Re-apply the accent default (it follows the theme when the user hasn't
    // set an explicit accent).
    crate::palette::set_accent(self.config.accent_rgb());
    self.config.save();
    self.redraw();
    self.set_status(format!("theme: {}", id.as_str()));
}
```

- [ ] **Step 5: Wire the command in `dispatch.rs`**

In `crates/crew-app/src/dispatch.rs`, add to the exact-match arm (next to `"reload" => ...`):

```rust
"theme" => self.set_theme_cmd(""),
```

and in the `other` arm (next to the other `strip_prefix` handlers):

```rust
} else if let Some(t) = other.strip_prefix("theme ") {
    self.set_theme_cmd(t.trim());
```

- [ ] **Step 6: Add `/theme` to the command palette**

In `crates/crew-app/src/suggest.rs`, add an entry to the `COMMANDS: &[Cmd]` list right after the `/reload` entry:

```rust
Cmd {
    name: "/theme",
    desc: "Switch theme (/theme [paper-light|paper-dark])",
},
```

- [ ] **Step 7: Run the tests**

Run: `cargo test -p crew-app`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/crew-app
git commit -m "feat(crew-app): /theme command switches and persists the theme live"
```

---

## Task 7: `Ctrl-Shift-L` hotkey to toggle themes

**Files:**
- Modify: `crates/crew-app/src/toggles.rs` (new `toggle_theme`)
- Modify: `crates/crew-app/src/keys.rs` (key handling, near the Ctrl+Tab block ~line 62)

**Interfaces:**
- Consumes: `CrewApp::set_theme_cmd` (from Task 6), `crew_theme::current_id`.
- Produces: `CrewApp::toggle_theme(&mut self)` — flips between the two presets via `set_theme_cmd`.

- [ ] **Step 1: Write the failing test**

Add to `crates/crew-app/src/toggles.rs` tests (mirroring `toggle_zoom_flips`):

```rust
#[test]
fn toggle_theme_flips() {
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut app = crate::app::CrewApp::default();
    app.toggle_theme();
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperLight);
    app.toggle_theme();
    assert_eq!(crew_theme::current_id(), crew_theme::ThemeId::PaperDark);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-app toggle_theme_flips`
Expected: FAIL — no `toggle_theme`.

- [ ] **Step 3: Implement `toggle_theme`**

Add to `crates/crew-app/src/toggles.rs` (inside `impl CrewApp`):

```rust
/// Flip between the two paper themes (Ctrl+Shift+L). Reuses `set_theme_cmd`
/// so it persists and repaints exactly like the `/theme` command.
pub(crate) fn toggle_theme(&mut self) {
    let next = match crew_theme::current_id() {
        crew_theme::ThemeId::PaperDark => crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::PaperLight => crew_theme::ThemeId::PaperDark,
    };
    self.set_theme_cmd(next.as_str());
}
```

- [ ] **Step 4: Wire the hotkey in `keys.rs`**

In `crates/crew-app/src/keys.rs`, add a block right after the Ctrl+Tab block (before the "Super-chords" block, ~line 80). Match `l`/`L` case-insensitively since Shift uppercases the logical key:

```rust
// Ctrl+Shift+L toggles the paper light/dark theme.
if event.state.is_pressed()
    && mstate.control_key()
    && mstate.shift_key()
    && matches!(&event.logical_key, Key::Character(s) if s.eq_ignore_ascii_case("l"))
{
    self.toggle_theme();
    return;
}
```

(`self.toggle_theme()` already calls `redraw()` via `set_theme_cmd`, so no extra `self.redraw()` is needed.)

- [ ] **Step 5: Run the tests**

Run: `cargo test -p crew-app`
Expected: PASS.

- [ ] **Step 6: Build the whole workspace**

Run: `cargo build`
Expected: clean build.

- [ ] **Step 7: Commit**

```bash
git add crates/crew-app
git commit -m "feat(crew-app): Ctrl+Shift+L toggles the paper light/dark theme"
```

---

## Task 8: Manual verification + docs note

**Files:**
- Modify: `README` or config docs if a theme/option list exists (optional; skip if none).

- [ ] **Step 1: Full test + lint**

Run: `cargo test && cargo fmt --check && cargo clippy --workspace`
Expected: all pass (the pre-commit hook also runs fmt + check).

- [ ] **Step 2: Launch and eyeball**

Run the app. Verify:
- Starts in `paper-dark` (warm charcoal page, dim parchment text), not pure black.
- `Ctrl+Shift+L` instantly flips to `paper-light` (warm off-white) and back — including the inside of terminal panes (run e.g. `ls --color` or `git status` and confirm output recolours to muted ink tones, stays readable).
- `/theme paper-light` and `/theme paper-dark` work; `/theme bogus` shows the error status; `/theme` with no arg reports the current theme.
- A user `accent = "#xxxxxx"` in `~/.config/crew/config.toml` still overrides the theme's default accent; with no `accent`, the accent matches the theme default.
- The choice persists across a restart (set light, quit, relaunch → still light).

- [ ] **Step 3: Commit any doc tweak**

```bash
git add -A
git commit -m "docs(crew): note paper themes and /theme command"
```

(Skip the commit if there was no doc to update.)

---

## Notes for the implementer

- **Field-name discipline:** the `Theme` field names defined in Task 1 are used verbatim in Tasks 2–4. If you rename a field, update every `theme().<field>` reference.
- **`color_opt`/ratatui mapping in `tui.rs`** is unchanged except the two default constants; ratatui `Color::Rgb` paths still pass through.
- **Deferred (NOT in this plan):** the `paper_texture` shader grain/vignette (Phase 2 in the spec). Do not add it here.
- **Why `redraw()` is enough for a live switch:** `build_frame()` rebuilds every `PaneScene` each frame and `build_scene` runs per frame, so all colours are resolved live from `theme()`; there is no cached scene to invalidate.
