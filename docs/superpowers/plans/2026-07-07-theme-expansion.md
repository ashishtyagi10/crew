# Theme Expansion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Four new dark themes, dark-only random rotation, Medium-weight text on light themes, and 3× newsprint grain on light themes.

**Architecture:** All behavior derives from two new per-theme data fields (`dark: bool`, `grain: f32`) on the `Theme` struct in crew-theme. Random mode filters its pool on `dark`; crew-render (which already depends on crew-theme and reads `crew_theme::theme()` per frame) picks the base font weight in `CellGrid::set_scene` and the effective grain in `Renderer::frame` from the active theme's fields. Spec: `docs/superpowers/specs/2026-07-07-theme-expansion-design.md`.

**Tech Stack:** Rust workspace; crew-theme (no deps), crew-render (wgpu/glyphon), existing test suites. No new dependencies.

## Global Constraints

- Source files ≤200 lines where practical (test files exempt); this plan splits `crew-theme/src/lib.rs` for that reason — do not grow it back.
- No new dependencies in any crate.
- TDD every behavior change: write the failing test, run it, watch it fail, implement, watch it pass.
- `cargo fmt` before every commit (pre-commit hook runs fmt check + cargo check).
- Weight values: dark themes → `400`, light themes → `500`; explicit bold cells always `Weight::BOLD` (700).
- Grain values: `grain: 1.0` on every dark theme, `grain: 3.0` on `paper-light`.
- Persistence ids (`as_u8`/`from_u8`): existing 0–4 unchanged; sepia-dark=5, midnight-ink=6, graphite=7, crt-violet=8.
- `ALL_THEMES` cycle order: `paper-dark, paper-light, sepia-dark, midnight-ink, graphite, crt-green, crt-amber, crt-blue, crt-violet`.
- New palettes MUST pass the existing `contrast_thresholds` test unchanged (it is the arbiter). If a listed RGB value fails an assertion, brighten that colour minimally (keep the hue) until it passes — do not loosen the test.
- Work on branch `feat/theme-expansion` off `main`.

---

### Task 1: Split crew-theme into focused files (pure refactor)

**Files:**
- Modify: `crates/crew-theme/src/lib.rs`
- Create: `crates/crew-theme/src/presets_paper.rs`
- Create: `crates/crew-theme/src/presets_crt.rs`
- Create: `crates/crew-theme/src/lib_tests.rs`

**Interfaces:**
- Consumes: current `lib.rs` (714 lines: Theme struct, 5 preset statics, ThemeId enum + runtime state, inline `mod tests`).
- Produces: identical public API (`Theme`, `PAPER_DARK`, `PAPER_LIGHT`, `CRT_GREEN`, `CRT_AMBER`, `CRT_BLUE`, `contrast_ratio`, `ThemeId`, `ALL_THEMES`, `set_theme`, `current_id`, `theme`, `is_random`, `random_pick`, `set_random`, `tick_random`, `cycle_next`, `ROTATE_MS`). Tasks 2–3 edit these files.

This is a move-only refactor — the existing test suite is the safety net; no new tests.

- [ ] **Step 1: Run the suite to record green baseline**

Run: `cargo test -p crew-theme`
Expected: all pass (13 tests).

- [ ] **Step 2: Move the presets and tests out of lib.rs**

- Create `crates/crew-theme/src/presets_paper.rs`: move the `PAPER_DARK` and `PAPER_LIGHT` statics (with their doc comments) verbatim. Top of file: `//! Paper-family presets: ink on paper, dark and light.` and `use crate::Theme;`. Statics keep `pub`.
- Create `crates/crew-theme/src/presets_crt.rs`: move `CRT_GREEN`, `CRT_AMBER`, `CRT_BLUE` verbatim. Top: `//! CRT-family presets: neon phosphor tubes.` and `use crate::Theme;`.
- Create `crates/crew-theme/src/lib_tests.rs`: move the entire body of `mod tests` (everything between `mod tests {` and its closing brace, including `use super::*;` and the `guard()` helper) verbatim.
- In `lib.rs`, replace the moved statics with:

```rust
mod presets_crt;
mod presets_paper;
pub use presets_crt::{CRT_AMBER, CRT_BLUE, CRT_GREEN};
pub use presets_paper::{PAPER_DARK, PAPER_LIGHT};
```

and replace the inline test module with:

```rust
#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
```

(The `#[path]`-file test pattern matches crew-app/crew-render convention, e.g. `celltext.rs` → `celltext_tests.rs`.)

- [ ] **Step 3: Verify identical behavior**

Run: `cargo fmt && cargo test -p crew-theme && cargo check --workspace`
Expected: same 13 tests pass; workspace check clean (no other crate sees any API change).

- [ ] **Step 4: Commit**

```bash
git add crates/crew-theme/src/
git commit -m "refactor(crew-theme): split presets and tests out of lib.rs"
```

### Task 2: `dark`/`grain` theme fields + dark-only random pool

**Files:**
- Modify: `crates/crew-theme/src/lib.rs` (Theme struct, `ThemeId::is_dark`, `random_pick`)
- Modify: `crates/crew-theme/src/presets_paper.rs`, `crates/crew-theme/src/presets_crt.rs` (field values)
- Test: `crates/crew-theme/src/lib_tests.rs`

**Interfaces:**
- Consumes: Task 1's file layout.
- Produces: `Theme { pub dark: bool, pub grain: f32, ... }`; `ThemeId::is_dark(self) -> bool`; `random_pick` filtered to dark themes. Task 3 sets these fields on new presets; Task 4 reads `theme().dark` and `theme().grain` from crew-render.

- [ ] **Step 1: Write the failing tests**

Append to `lib_tests.rs`:

```rust
#[test]
fn dark_flag_matches_page_bg_luminance() {
    // The `dark` field is design data, but it may never contradict the
    // palette: WCAG relative luminance of page_bg < 0.5 ⇔ dark.
    let lin = |c: u8| -> f32 {
        let x = c as f32 / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    for id in ALL_THEMES {
        let t = id.theme();
        let lum =
            0.2126 * lin(t.page_bg.0) + 0.7152 * lin(t.page_bg.1) + 0.0722 * lin(t.page_bg.2);
        assert_eq!(
            t.dark,
            lum < 0.5,
            "{}: dark={} but page_bg luminance={lum:.3}",
            id.as_str(),
            t.dark
        );
    }
}

#[test]
fn grain_is_newsprint_on_light_and_subtle_on_dark() {
    for id in ALL_THEMES {
        let t = id.theme();
        let want = if t.dark { 1.0 } else { 3.0 };
        assert_eq!(t.grain, want, "{}: grain", id.as_str());
    }
}

#[test]
fn random_pick_only_returns_dark_themes() {
    // Random rotation must never land on a light theme, from any start.
    for current in ALL_THEMES {
        for seed in [0u64, 1, 2, 42, 999, 600_000, u64::MAX, 123_456_789] {
            let picked = random_pick(current, seed);
            assert!(
                picked.is_dark(),
                "seed {seed} from {} picked light theme {}",
                current.as_str(),
                picked.as_str()
            );
            assert_ne!(picked, current);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-theme`
Expected: compile error — `Theme` has no field `dark` (the struct change is the failure; that's the RED state for a data-shape change).

- [ ] **Step 3: Implement**

In `lib.rs`, add to the END of the `Theme` struct (after `ansi`):

```rust
    /// Whether this is a dark theme (dark page, light ink). Drives the
    /// random-rotation pool, the light-theme text weight, and grain.
    pub dark: bool,
    /// Grain amplitude multiplier for the paper-texture pass, relative to
    /// the user's configured `paper_grain`. 1.0 on dark themes; 3.0 on
    /// light themes for a visible newsprint texture.
    pub grain: f32,
```

In every preset add the two fields (after `ansi: [...]`):
- `PAPER_LIGHT`: `dark: false, grain: 3.0,`
- `PAPER_DARK`, `CRT_GREEN`, `CRT_AMBER`, `CRT_BLUE`: `dark: true, grain: 1.0,`

In `lib.rs`, add to `impl ThemeId` (after `describe`):

```rust
    /// Whether this theme is dark — see [`Theme::dark`].
    pub fn is_dark(self) -> bool {
        self.theme().dark
    }
```

Change `random_pick`'s filter (the only line that changes; update its doc
comment to say "Pick a DARK theme … the pool excludes light themes and
`current`"):

```rust
        .filter(|&t| t.is_dark() && t != current)
```

And update its "never empty" comment: the pool is all dark themes minus
possibly `current` — with 4 dark themes now (8 after Task 3) it is never
empty.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo fmt && cargo test -p crew-theme`
Expected: all pass, including the three new tests and the untouched
`random_pick_never_returns_current_and_is_deterministic`.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-theme/src/
git commit -m "feat(crew-theme): dark/grain theme fields; random mode rotates dark themes only"
```

### Task 3: Four new dark themes

**Files:**
- Modify: `crates/crew-theme/src/lib.rs` (ThemeId variants + mappings + ALL_THEMES)
- Modify: `crates/crew-theme/src/presets_paper.rs` (SEPIA_DARK, MIDNIGHT_INK, GRAPHITE)
- Modify: `crates/crew-theme/src/presets_crt.rs` (CRT_VIOLET)
- Test: `crates/crew-theme/src/lib_tests.rs`, `crates/crew-app/src/suggest_tests.rs`

**Interfaces:**
- Consumes: Task 2's `dark`/`grain` fields.
- Produces: `ThemeId::{SepiaDark, MidnightInk, Graphite, CrtViolet}`; names `"sepia-dark"`, `"midnight-ink"`, `"graphite"`, `"crt-violet"`; `ALL_THEMES: [ThemeId; 9]`.

- [ ] **Step 1: Write the failing tests**

In `lib_tests.rs`, REPLACE the body of `cycle_next_walks_all_themes_then_random_then_wraps` with:

```rust
    let _g = guard();
    set_random(false, 0);
    set_theme(ThemeId::PaperDark);
    // Starting at paper-dark, each call steps to the next fixed theme...
    for want in [
        "paper-light",
        "sepia-dark",
        "midnight-ink",
        "graphite",
        "crt-green",
        "crt-amber",
        "crt-blue",
        "crt-violet",
    ] {
        assert_eq!(cycle_next(1), want);
    }
    // ...then from the last fixed theme it enters random mode...
    assert_eq!(cycle_next(5), "random");
    assert!(is_random());
    // ...and from random it wraps back to the first fixed theme, off.
    assert_eq!(cycle_next(6), "paper-dark");
    assert!(!is_random());
    assert_eq!(current_id(), ThemeId::PaperDark);
    set_random(false, 0);
    set_theme(ThemeId::PaperDark);
```

Append a new test:

```rust
#[test]
fn u8_mapping_round_trips_all_nine_ids() {
    // Persistence mapping: every id survives as_u8 → from_u8 (via the
    // set_theme/current_id atomics), and the new ids extend the mapping
    // without renumbering the original five.
    let _g = guard();
    for id in ALL_THEMES {
        set_theme(id);
        assert_eq!(current_id(), id, "{} lost by u8 round-trip", id.as_str());
    }
    assert_eq!(ThemeId::from_u8(5), ThemeId::SepiaDark);
    assert_eq!(ThemeId::from_u8(6), ThemeId::MidnightInk);
    assert_eq!(ThemeId::from_u8(7), ThemeId::Graphite);
    assert_eq!(ThemeId::from_u8(8), ThemeId::CrtViolet);
    set_theme(ThemeId::PaperDark);
}
```

(`from_u8` is private; the test module lives inside the crate so it can call
it directly.)

In `crates/crew-app/src/suggest_tests.rs`, extend the name array in
`theme_space_lists_all_themes_as_runnable_values` to also cover the new
themes:

```rust
    for name in [
        "paper-dark",
        "paper-light",
        "sepia-dark",
        "midnight-ink",
        "graphite",
        "crt-green",
        "crt-amber",
        "crt-blue",
        "crt-violet",
    ] {
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-theme`
Expected: compile error — no variant `SepiaDark` (RED for an enum extension).

- [ ] **Step 3: Add the presets**

Append to `presets_paper.rs`:

```rust
/// **Sepia dark**: dark coffee-brown paper with warm cream ink — the paper
/// family's "aged newsprint at night" page.
pub static SEPIA_DARK: Theme = Theme {
    page_bg: (24, 17, 11),
    ink: (241, 229, 205),
    text_muted: (208, 192, 164),
    term_fg: (241, 229, 205),
    term_bg: (24, 17, 11),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (92, 74, 55),
    border_focused: (216, 192, 150),
    border_thickness: 2.5,
    legend_off: (170, 152, 124),
    accent_default: (235, 190, 120),
    status_fg: (235, 195, 120),
    broadcast: (210, 150, 180),
    activity: (150, 175, 205),
    bell: (235, 195, 120),
    dim: (140, 124, 100),
    placeholder: (128, 113, 92),
    hint_fg: (150, 134, 108),
    find_hl_bg: (80, 62, 24),
    ansi: [
        (100, 88, 72),   // 0  black -> warm grey
        (235, 110, 85),  // 1  red
        (150, 215, 110), // 2  green
        (235, 200, 95),  // 3  yellow
        (130, 180, 230), // 4  blue
        (215, 145, 205), // 5  magenta
        (120, 215, 205), // 6  cyan
        (228, 218, 200), // 7  white -> warm light grey
        (150, 136, 116), // 8  bright black
        (255, 135, 105), // 9  bright red
        (180, 235, 130), // 10 bright green
        (255, 220, 115), // 11 bright yellow
        (155, 200, 250), // 12 bright blue
        (235, 170, 225), // 13 bright magenta
        (145, 240, 225), // 14 bright cyan
        (248, 240, 225), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};

/// **Midnight ink**: deep navy page with cool off-white ink — a calm
/// blue-black newspaper.
pub static MIDNIGHT_INK: Theme = Theme {
    page_bg: (10, 14, 28),
    ink: (232, 238, 248),
    text_muted: (185, 196, 215),
    term_fg: (232, 238, 248),
    term_bg: (10, 14, 28),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (66, 76, 100),
    border_focused: (200, 214, 235),
    border_thickness: 2.5,
    legend_off: (140, 152, 175),
    accent_default: (150, 190, 245),
    status_fg: (235, 200, 120),
    broadcast: (200, 155, 215),
    activity: (130, 180, 225),
    bell: (235, 200, 120),
    dim: (110, 120, 140),
    placeholder: (100, 110, 130),
    hint_fg: (120, 131, 152),
    find_hl_bg: (50, 62, 100),
    ansi: [
        (90, 96, 110),   // 0  black -> cool grey
        (240, 110, 100), // 1  red
        (135, 215, 125), // 2  green
        (230, 200, 100), // 3  yellow
        (115, 175, 240), // 4  blue
        (205, 145, 220), // 5  magenta
        (105, 215, 220), // 6  cyan
        (220, 226, 236), // 7  white -> cool light grey
        (135, 143, 158), // 8  bright black
        (255, 135, 120), // 9  bright red
        (165, 235, 145), // 10 bright green
        (250, 220, 120), // 11 bright yellow
        (140, 195, 255), // 12 bright blue
        (230, 168, 240), // 13 bright magenta
        (130, 240, 240), // 14 bright cyan
        (245, 248, 252), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};

/// **Graphite**: neutral charcoal page with soft white ink — a gentler,
/// lower-glare paper-dark.
pub static GRAPHITE: Theme = Theme {
    page_bg: (28, 28, 30),
    ink: (226, 226, 228),
    text_muted: (183, 183, 186),
    term_fg: (226, 226, 228),
    term_bg: (28, 28, 30),
    // Focus-led border hierarchy, as in paper-dark.
    border_normal: (85, 85, 88),
    border_focused: (215, 215, 218),
    border_thickness: 2.5,
    legend_off: (150, 150, 154),
    accent_default: (222, 222, 225),
    status_fg: (230, 195, 125),
    broadcast: (198, 152, 188),
    activity: (142, 175, 208),
    bell: (230, 195, 125),
    dim: (130, 130, 134),
    placeholder: (120, 120, 124),
    hint_fg: (140, 140, 144),
    find_hl_bg: (75, 68, 28),
    ansi: [
        (110, 110, 113), // 0  black -> mid grey
        (235, 110, 95),  // 1  red
        (145, 220, 115), // 2  green
        (235, 200, 95),  // 3  yellow
        (125, 182, 235), // 4  blue
        (215, 145, 215), // 5  magenta
        (115, 220, 215), // 6  cyan
        (222, 222, 225), // 7  white -> light grey
        (145, 145, 149), // 8  bright black
        (255, 135, 115), // 9  bright red
        (175, 240, 135), // 10 bright green
        (255, 220, 115), // 11 bright yellow
        (150, 202, 255), // 12 bright blue
        (235, 168, 235), // 13 bright magenta
        (140, 245, 235), // 14 bright cyan
        (246, 246, 248), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};
```

Append to `presets_crt.rs`:

```rust
/// **Neon violet phosphor** (electrified): ultraviolet orchid on a
/// near-black tube — the fourth phosphor, glowing purple.
pub static CRT_VIOLET: Theme = Theme {
    page_bg: (12, 4, 18),
    ink: (232, 170, 255),
    text_muted: (205, 140, 235),
    term_fg: (232, 170, 255),
    term_bg: (12, 4, 18),
    // Unfocused borders sit back (focus-led hierarchy, as in paper-dark).
    border_normal: (88, 50, 110),
    border_focused: (225, 160, 255),
    border_thickness: 2.5,
    legend_off: (170, 115, 200),
    accent_default: (240, 180, 255),
    status_fg: (245, 185, 250),
    broadcast: (255, 150, 200),
    activity: (190, 140, 255),
    bell: (255, 190, 240),
    dim: (120, 78, 145),
    placeholder: (135, 88, 162),
    hint_fg: (150, 100, 180),
    find_hl_bg: (60, 25, 85),
    ansi: [
        (55, 35, 75),    // 0  black
        (255, 140, 200), // 1  red
        (190, 150, 255), // 2  green
        (235, 180, 255), // 3  yellow
        (160, 140, 255), // 4  blue
        (230, 140, 255), // 5  magenta
        (200, 160, 255), // 6  cyan
        (230, 200, 250), // 7  white
        (140, 95, 175),  // 8  bright black
        (255, 160, 220), // 9  bright red
        (210, 170, 255), // 10 bright green
        (245, 200, 255), // 11 bright yellow
        (180, 160, 255), // 12 bright blue
        (240, 160, 255), // 13 bright magenta
        (215, 180, 255), // 14 bright cyan
        (245, 225, 255), // 15 bright white
    ],
    dark: true,
    grain: 1.0,
};
```

- [ ] **Step 4: Register the variants in lib.rs**

- Re-exports: `pub use presets_paper::{GRAPHITE, MIDNIGHT_INK, PAPER_DARK, PAPER_LIGHT, SEPIA_DARK};` and `pub use presets_crt::{CRT_AMBER, CRT_BLUE, CRT_GREEN, CRT_VIOLET};`
- Enum: add variants `SepiaDark, MidnightInk, Graphite, CrtViolet`.
- `ALL_THEMES` becomes `[ThemeId; 9]` in the Global Constraints order.
- `as_str`: `"sepia-dark"`, `"midnight-ink"`, `"graphite"`, `"crt-violet"`.
- `describe`: SepiaDark → `"dark sepia paper (warm cream ink)"`, MidnightInk → `"deep navy page, cool off-white ink"`, Graphite → `"soft charcoal paper (gentle dark)"`, CrtViolet → `"neon violet phosphor CRT"`.
- `from_name`: the four new names.
- `theme()`: map to the new statics.
- `as_u8`: SepiaDark=5, MidnightInk=6, Graphite=7, CrtViolet=8. `from_u8`: 5→SepiaDark, 6→MidnightInk, 7→Graphite, 8→CrtViolet (default arm stays PaperDark).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo fmt && cargo test -p crew-theme && cargo test -p crew-app suggest`
Expected: ALL crew-theme tests pass — the contrast, no-pure-black/white,
term_bg==page_bg, dark-flag, and grain gates now sweep 9 themes. If a
`contrast_thresholds` assertion fails for a new palette, brighten the named
colour minimally (keep the hue) and re-run; do NOT touch the thresholds.
The crew-app suggest tests pass with the extended picker list.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-theme/src/ crates/crew-app/src/suggest_tests.rs
git commit -m "feat(crew-theme): four new dark themes — sepia-dark, midnight-ink, graphite, crt-violet"
```

### Task 4: Theme-driven text weight and grain in crew-render

**Files:**
- Modify: `crates/crew-render/src/celltext.rs` (FontParams + fill_rich_text)
- Modify: `crates/crew-render/src/cellgrid.rs:135-140` (FontParams construction)
- Modify: `crates/crew-render/src/scenecache.rs:16-30` (pane_sig)
- Modify: `crates/crew-render/src/renderer.rs:115` (effective grain)
- Test: `crates/crew-render/src/celltext_tests.rs`, `crates/crew-render/src/scene_sig_tests.rs`, `crates/crew-render/src/scene_tests.rs` (fixture updates)

**Interfaces:**
- Consumes: `crew_theme::theme().dark` and `.grain` (Task 2). crew-render already depends on crew-theme (see `renderer.rs` reading `theme().page_bg`).
- Produces: `FontParams { pub weight: u16, ... }` — the base weight for non-bold text; bold cells stay `Weight::BOLD`.

- [ ] **Step 1: Write the failing tests**

In `crates/crew-render/src/scene_sig_tests.rs`, add `weight: 400,` to the
`params()` fixture, and append:

```rust
#[test]
fn pane_sig_changes_when_base_weight_changes() {
    // A light-theme switch changes only FontParams.weight; the signature
    // must change so cached buffers shaped at the old weight are rebuilt.
    let p = pane(vec![cell(0, 0, 'a', (1, 2, 3))], false, false);
    let normal = params();
    let mut medium = params();
    medium.weight = 500;
    assert_ne!(
        pane_sig(&p, 10, 2, &normal),
        pane_sig(&p, 10, 2, &medium),
        "weight is part of the signature"
    );
}
```

In `crates/crew-render/src/scene_tests.rs`, add `weight: 400,` to its
`params()` fixture.

In `crates/crew-render/src/celltext_tests.rs`, the existing `params(...)`
helper(s) and inline `FontParams { ... }` literals gain `weight: 400,`.
Append a snapping test mirroring `bold_glyphs_snap_to_the_same_cell_advance`
(Medium faces may have different natural advances; the cell quantum must
still hold):

```rust
#[test]
fn medium_weight_glyphs_snap_to_the_same_cell_advance() {
    let style = |col: u16, c: char| CellView {
        col,
        row: 0,
        c,
        fg: (200, 200, 200),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
    };
    let mut fs = FontSystem::new();
    let cells = vec![style(0, 'W'), style(1, 'i'), style(2, 'm'), style(3, '0')];
    let (cell_w, cell_h) = cell_metrics(14.0);
    let p = FontParams {
        font_size: 14.0,
        line_height: cell_h,
        cell_w,
        family: None,
        weight: 500,
    };
    let buf = build_pane_buffer(&mut fs, &cells, 4, 1, 4.0 * cell_w, cell_h, &p);
    let runs: Vec<_> = buf.layout_runs().collect();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].glyphs.len(), 4, "four columns shape to four glyphs");
    for g in runs[0].glyphs {
        let cols = g.x / cell_w;
        assert!(
            (cols - cols.round()).abs() < 1e-3,
            "medium glyph at x={} is off the {cell_w}px grid",
            g.x
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-render`
Expected: compile error — `FontParams` has no field `weight` (RED for the
data-shape change).

- [ ] **Step 3: Implement**

`celltext.rs`:

- `FontParams` gains (after `family`):

```rust
    /// Base weight for non-bold text (CSS scale; 400 normal, 500 medium).
    /// Light themes use 500 so ink reads crisp on a bright page; bold cells
    /// always shape at `Weight::BOLD` regardless.
    pub weight: u16,
```

- In `build_pane_buffer`, pass the weight through:
  `fill_rich_text(&mut buffer, font_system, cells, cols, rows, &params.family, params.weight);`
- `fill_rich_text` signature gains `weight: u16`. Inside:
  - `let base = Weight(weight);`
  - `let default_attrs = Attrs::new().family(fam).weight(base);`
  - In the styled arm replace the bold `if`:

```rust
                RunKey::Styled(fg, bold, italic) => {
                    let mut a = Attrs::new()
                        .family(fam)
                        .color(Color::rgb(fg.0, fg.1, fg.2))
                        .weight(if *bold { Weight::BOLD } else { base });
                    if *italic {
                        a = a.style(Style::Italic);
                    }
                    a
                }
```

`cellgrid.rs` (`set_scene`, line ~135): the FontParams literal gains

```rust
            // Light themes render base text at Medium for crisp ink on a
            // bright page; per-frame theme read, same pattern as page_bg
            // in renderer.rs.
            weight: if crew_theme::theme().dark { 400 } else { 500 },
```

`scenecache.rs` (`pane_sig`): extend the hashed tuple with the weight —

```rust
    (
        params.font_size.to_bits(),
        params.line_height.to_bits(),
        params.cell_w.to_bits(),
        params.weight,
    )
        .hash(&mut h);
```

(match the existing tuple contents; just add `params.weight`).

`renderer.rs` (`frame`, the `update_uniform` call): pass the effective grain —

```rust
                self.paper_bg.update_uniform(
                    &self.gpu.queue,
                    bg_f32,
                    self.gpu.config.width as f32,
                    self.gpu.config.height as f32,
                    1.0,
                    // Newsprint: light themes multiply the user's grain knob
                    // (theme().grain = 3.0 there, 1.0 on darks).
                    self.paper_grain * crew_theme::theme().grain,
                );
```

Also update the doc comment on `set_paper_grain` to note the stored value is
the USER knob; the active theme's `grain` multiplies it at frame time.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo fmt && cargo test -p crew-render && cargo test --workspace`
Expected: all green, including the two new tests and every existing snapping/
sig/cache test.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-render/src/
git commit -m "feat(crew-render): light themes get Medium base weight and 3x newsprint grain"
```

### Task 5 (controller, inline): verify → merge → install → smoke

- [ ] `cargo test --workspace` green.
- [ ] Visual smoke via the screenshot harness (per `project-gui-verify-harness` memory): launch the dev build, `/theme paper-light`, screenshot — grain visibly stronger than on a dark theme, text reads heavier; `/theme random` — status shows a dark theme; wait is impractical for the 10-min rotation, rely on the unit tests for cadence.
- [ ] superpowers:finishing-a-development-branch (tests → options → user chose merge flow previously; ask).
- [ ] `cargo build --release` + install to `~/.local/bin/crew` if user wants, remind restart.

## Self-Review Notes

- Spec coverage: mechanism fields (Task 2), 4 themes + registration + picker lists (Task 3), dark-only random (Task 2), weight + cache sig + grain (Task 4), file split (Task 1), visual pass (Task 5). Out-of-scope items untouched.
- Palette values were sanity-checked against the WCAG gates by hand; `contrast_thresholds` is the enforcement, and Task 3 Step 5 tells the implementer how to resolve a marginal failure without weakening tests.
- Type consistency: `weight: u16` everywhere; `Weight(weight)` (cosmic-text tuple struct) in celltext; `params.weight` hashed as a plain u16.
