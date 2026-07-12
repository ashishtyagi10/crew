# Welcome Screen: Rotating ASCII Globe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the welcome screen's figlet "CREW" banner, its per-column shimmer, and the animated dev-at-terminal figure with a procedurally raycast, smoothly rotating ASCII globe — continents shaded by illumination — with the tagline and keyboard hint centred beneath it.

**Architecture:** A new pure, cell-based `welcomeglobe::globe()` function orthographically back-projects every cell in a `w`×`h` box onto a unit sphere (correcting for the terminal's ~2:1 cell aspect so the disc reads as a circle), looks up an embedded coarse 48×24 lon/lat continent bitmap at the phase-rotated longitude, and shades land/sea into fixed density charsets by upper-left Lambertian illumination. `welcome.rs` picks the largest globe size that centres in the pane (default 44×22, floor 16×8) and stacks it above the tagline/hint, driven by the same already-throttled `tick` the idle redraw loop already supplies; `welcomeart.rs` and the figlet banner are deleted outright.

**Tech Stack:** Rust (`crew-app` crate), `crew_render::CellView` (existing cell-grid renderer type — fields `col, row, c, fg, bg, bold, italic`), `crew_theme::theme()` for land/sea/hint/dim colors. No new crates or dependencies.

## Global Constraints

- `welcomeglobe::globe()` is pure and deterministic: identical inputs (box, `phase`, colors) always produce identical output cells, in the same order — no time-of-day, RNG, or hidden state.
- Colors are theme-driven only: land draws in `t.ink`, sea in `t.text_muted` — no hardcoded RGB triples for globe shading.
- Rotation runs at the existing ~20 fps idle-redraw cadence (`ANIM_DIV = 3`, unchanged) via `phase = tick as f32 * 0.05`, where `tick` is the same already-divided value `welcome_cells_animated` already receives from `render.rs` — no new timers.
- Default globe size is 44×22 cells; when the pane is smaller it scales down to the largest fitting even width ≥ 16 (height ≥ 8, i.e. `width/2`); below that it falls back to the existing spaced single-line "CREW" — kept structurally verbatim (layout math unchanged; see Self-Review Notes for how this reconciles with the shimmer's removal).
- Cell aspect is 2:1 (a cell is twice as tall as wide) — the disc math normalizes the vertical axis by half the row extent (not the full extent) so a `w × w/2` box projects as a circle, not a flattened oval.
- Zero `cargo check` warnings across the workspace; rustfmt clean (pre-commit hook enforces both). Note: `welcomeglobe.rs`'s pub surface is briefly dead-code-unused between Task 1 and Task 2 — the zero-warnings gate is asserted once, at the end of Task 2, after `welcome.rs` wires it in (verified empirically while planning; see Self-Review Notes).

---

### Task 1: `welcomeglobe.rs` — the earth bitmap, sphere math, and the pure `globe()` renderer

**Files:**
- Create: `crates/crew-app/src/welcomeglobe.rs`
- Modify: `crates/crew-app/src/main.rs` (insert `mod welcomeglobe;` after line 123, before `mod windowtitle;`)

**Interfaces:**
- Produces: `pub const GLOBE_W: u16`, `pub const GLOBE_H: u16`, `pub const GLOBE_MIN_W: u16`, `pub const GLOBE_MIN_H: u16`, and `pub fn globe(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16, phase: f32, land: (u8,u8,u8), sea: (u8,u8,u8), bg: (u8,u8,u8))` — all consumed by Task 2.
- Consumes: `crew_render::CellView` only.

- [ ] **Step 1: Write the failing tests (+ data the tests need)**

Register the module in `crates/crew-app/src/main.rs` — change lines 122-124 from:

```rust
mod welcome;
mod welcomeart;
mod windowtitle;
```

to:

```rust
mod welcome;
mod welcomeart;
mod welcomeglobe;
mod windowtitle;
```

(`welcomeart` stays for now — Task 2 removes it.)

Create `crates/crew-app/src/welcomeglobe.rs` with the bitmap data, the two shading charsets, and the test module — but *not yet* `is_land`, `shade_char`, `light_dir`, or `globe` themselves, so the tests fail to compile against missing symbols:

```rust
//! Procedurally rendered ASCII globe for the welcome screen: an orthographic,
//! rotating projection of a coarse embedded continent bitmap, shaded by a
//! fixed upper-left light so the sphere reads as a ball, not a flat map.
use crew_render::CellView;
use std::f32::consts::{PI, TAU};

/// Default globe box size in cells. 44 wide × 22 tall keeps the disc a
/// circle, not an oval, given the terminal's ~2:1 cell aspect (a cell is
/// twice as tall as wide).
pub const GLOBE_W: u16 = 44;
pub const GLOBE_H: u16 = GLOBE_W / 2;
/// Smallest globe box still worth drawing; below this `welcome.rs` falls
/// back to the spaced single-line "CREW".
pub const GLOBE_MIN_W: u16 = 16;
pub const GLOBE_MIN_H: u16 = GLOBE_MIN_W / 2;

const EARTH_W: usize = 48;
const EARTH_H: usize = 24;

/// Earth bitmap: 48 longitude columns × 24 latitude rows, 1 bit/cell, packed
/// MSB-first into the low 48 bits of each `u64` (bit 47 = column 0, bit 0 =
/// column 47). Row 0 is the north-pole band, row 23 the south-pole band.
/// Column 0 is an arbitrary longitude reference frame — `globe`'s `phase`
/// rotates the visible slice through it every frame, so its absolute
/// meaning never surfaces. Equirectangular, coarse continents (`#` = land):
///
/// ```text
/// ................................................
/// ......##########..#####.........................
/// ..####################........##################
/// .#####################..########################
/// ...##################.##########################
/// ......############....#########################.
/// ......###########.....######################....
/// .......#########......######################....
/// .......#########.....######################.....
/// ........#####........######################.....
/// ........#########....######################.....
/// ............#####.....##########.###.######.....
/// ..............######.....#######.....######.....
/// ...............#####......######.......######...
/// ...............#####......######.......######...
/// ..............#####.......######.......######...
/// ..............####........###..........######...
/// ..............###......................######.##
/// ..............###.............................##
/// ..............##................................
/// ................................................
/// ................................................
/// ................................................
/// ................................................
/// ```
///
/// Top to bottom: Arctic islands + Greenland; North America tapering through
/// the Central American isthmus into South America, which tapers to a point
/// (Patagonia) near the bottom. The Old World mass carries Europe merging
/// into Siberia/Asia (Eurasia is genuinely one landmass), Africa hanging
/// beneath Europe, India and Indonesia as separate tendrils off Asia's
/// southern edge, and Australia + New Zealand as islands lower-right.
#[rustfmt::skip]
const EARTH: [u64; EARTH_H] = [
    0x000000000000, 0x03FF3E000000, 0x3FFFFC03FFFF, 0x7FFFFCFFFFFF,
    0x1FFFFBFFFFFF, 0x03FFC3FFFFFE, 0x03FF83FFFFF0, 0x01FF03FFFFF0,
    0x01FF07FFFFE0, 0x00F807FFFFE0, 0x00FF87FFFFE0, 0x000F83FF77E0,
    0x0003F07F07E0, 0x0001F03F01F8, 0x0001F03F01F8, 0x0003E03F01F8,
    0x0003C03801F8, 0x0003800001FB, 0x000380000003, 0x000300000000,
    0x000000000000, 0x000000000000, 0x000000000000, 0x000000000000,
];

/// Land shading, brightest (fully lit) to dimmest (grazing/shadowed).
const LAND_CHARS: [char; 5] = ['#', '%', '*', '+', '='];
/// Sea shading, brightest to dimmest — dimmest is a space (bg shows through).
const SEA_CHARS: [char; 3] = ['·', '.', ' '];

#[cfg(test)]
mod tests {
    use super::*;

    fn tuples(cells: &[CellView]) -> Vec<(u16, u16, char, (u8, u8, u8), (u8, u8, u8))> {
        cells.iter().map(|c| (c.col, c.row, c.c, c.fg, c.bg)).collect()
    }

    #[test]
    fn is_land_reads_the_bitmap() {
        assert!(!is_land(0, 0), "north pole band is all sea");
        assert!(is_land(10, 10), "row10 col10 should be North America");
        assert!(!is_land(10, 20), "row10 col20 should be the Atlantic gap");
        assert!(!is_land(EARTH_H, 0), "out-of-range row reads as sea");
        assert!(!is_land(0, EARTH_W), "out-of-range col reads as sea");
    }

    #[test]
    fn shade_char_spans_bright_to_dim() {
        assert_eq!(shade_char(&LAND_CHARS, 1.0), '#');
        assert_eq!(shade_char(&LAND_CHARS, 0.0), '=');
        assert_eq!(shade_char(&SEA_CHARS, 1.0), '·');
        assert_eq!(shade_char(&SEA_CHARS, 0.0), ' ');
    }

    #[test]
    fn globe_cells_stay_within_its_box() {
        let mut cells = Vec::new();
        globe(&mut cells, 3, 5, GLOBE_W, GLOBE_H, 0.0, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|c| {
            c.col >= 5 && c.col < 5 + GLOBE_W && c.row >= 3 && c.row < 3 + GLOBE_H
        }));
    }

    #[test]
    fn globe_corners_are_outside_the_disc() {
        for (w, h) in [(GLOBE_MIN_W, GLOBE_MIN_H), (GLOBE_W, GLOBE_H)] {
            let mut cells = Vec::new();
            globe(&mut cells, 0, 0, w, h, 0.0, (1, 1, 1), (2, 2, 2), (0, 0, 0));
            for (col, row) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
                assert!(
                    !cells.iter().any(|c| c.col == col && c.row == row),
                    "corner ({col},{row}) of {w}x{h} should be outside the disc"
                );
            }
        }
    }

    #[test]
    fn globe_is_deterministic() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        globe(&mut a, 0, 0, GLOBE_W, GLOBE_H, 1.23, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        globe(&mut b, 0, 0, GLOBE_W, GLOBE_H, 1.23, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        assert_eq!(tuples(&a), tuples(&b));
    }

    #[test]
    fn globe_rotation_changes_the_frame() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        globe(&mut a, 0, 0, GLOBE_W, GLOBE_H, 0.0, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        globe(&mut b, 0, 0, GLOBE_W, GLOBE_H, 0.5, (1, 1, 1), (2, 2, 2), (0, 0, 0));
        assert_ne!(tuples(&a), tuples(&b));
    }

    #[test]
    fn globe_shading_partitions_by_char_and_color() {
        let land = (9, 9, 9);
        let sea = (8, 8, 8);
        let mut cells = Vec::new();
        globe(&mut cells, 0, 0, GLOBE_W, GLOBE_H, 0.7, land, sea, (0, 0, 0));
        assert!(!cells.is_empty());
        for c in &cells {
            let is_land_cell = LAND_CHARS.contains(&c.c);
            let is_sea_cell = SEA_CHARS.contains(&c.c);
            assert!(is_land_cell != is_sea_cell, "{:?} must be exactly one of land/sea", c.c);
            if is_land_cell {
                assert_eq!(c.fg, land);
            } else {
                assert_eq!(c.fg, sea);
            }
        }
    }

    #[test]
    fn globe_visible_hemisphere_has_land_and_sea() {
        let land = (9, 9, 9);
        let sea = (8, 8, 8);
        let mut cells = Vec::new();
        globe(&mut cells, 0, 0, GLOBE_W, GLOBE_H, 0.0, land, sea, (0, 0, 0));
        assert!(cells.iter().any(|c| c.fg == land), "phase 0 hemisphere has no land");
        assert!(cells.iter().any(|c| c.fg == sea), "phase 0 hemisphere has no sea");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app welcomeglobe`
Expected: compile FAIL — `is_land`, `shade_char`, and `globe` not found in this scope.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/welcomeglobe.rs`, insert the following between the `SEA_CHARS` const and the `#[cfg(test)]` module:

```rust
/// True if latitude band `row` (`0..EARTH_H`) / longitude band `col`
/// (`0..EARTH_W`) is land. Out-of-range indices read as sea.
fn is_land(row: usize, col: usize) -> bool {
    if row >= EARTH_H || col >= EARTH_W {
        return false;
    }
    (EARTH[row] >> (EARTH_W - 1 - col)) & 1 != 0
}

/// Pick a shading character from `chars` (ordered brightest → dimmest) for
/// illumination `illum` in `[0, 1]`.
fn shade_char(chars: &[char], illum: f32) -> char {
    let n = chars.len();
    let idx = (((1.0 - illum.clamp(0.0, 1.0)) * n as f32) as usize).min(n - 1);
    chars[idx]
}

/// Unit-length light direction, fixed once: upper-left, tilted toward the
/// viewer, so the sphere reads with a highlight near its upper-left limb.
fn light_dir() -> (f32, f32, f32) {
    let (x, y, z) = (-0.5_f32, 0.6_f32, 0.65_f32);
    let len = (x * x + y * y + z * z).sqrt();
    (x / len, y / len, z / len)
}

/// Render a `w`×`h`-cell globe frame at rotation `phase` (radians) with its
/// top-left at `(top, left)`. Pure and deterministic: identical inputs
/// always produce identical cells, in the same order. Points outside the
/// projected disc emit no cell (the page shows through).
///
/// Projection: each cell maps to `(u, v)` in roughly `[-1, 1]`, normalized
/// by half-width and half-height separately so a `w × w/2` box (the 2:1 cell
/// aspect) reads as a circle. Points with `u²+v² > 1` are outside the disc.
/// Inside it, `zc = sqrt(1 - u² - v²)` completes a unit sphere point
/// `(u, v, zc)`; standard spherical unprojection gives `lat = asin(v)` and
/// `lon = atan2(u, zc)` (this holds for every `v`, not just the equator —
/// `zc` is exactly `cos(lat)` at any latitude). `phase` is added to `lon`
/// before wrapping to rotate the visible slice.
#[rustfmt::skip]
pub fn globe(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16,
             phase: f32, land: (u8, u8, u8), sea: (u8, u8, u8), bg: (u8, u8, u8)) {
    if w == 0 || h == 0 { return; }
    let cx = (w - 1) as f32 / 2.0;
    let cy = (h - 1) as f32 / 2.0;
    let hw = w as f32 / 2.0;
    let hh = h as f32 / 2.0;
    let (lx, ly, lz) = light_dir();

    for row in 0..h {
        for col in 0..w {
            let u = (col as f32 - cx) / hw;
            let v = (cy - row as f32) / hh; // flip: row 0 (top) -> v > 0 (north)
            let d2 = u * u + v * v;
            if d2 > 1.0 { continue; } // outside the disc: page shows through

            let zc = (1.0 - d2).max(0.0).sqrt();
            let lat = v.clamp(-1.0, 1.0).asin();
            let lon = (u.atan2(zc) + phase).rem_euclid(TAU);

            let erow = (((PI / 2.0 - lat) / PI) * EARTH_H as f32) as usize;
            let erow = erow.min(EARTH_H - 1);
            let ecol = ((lon / TAU) * EARTH_W as f32) as usize % EARTH_W;

            let illum = (u * lx + v * ly + zc * lz).clamp(0.0, 1.0);
            let (c, fg) = if is_land(erow, ecol) {
                (shade_char(&LAND_CHARS, illum), land)
            } else {
                (shade_char(&SEA_CHARS, illum), sea)
            };
            cells.push(CellView { col: left + col, row: top + row, c, fg, bg, bold: false, italic: false });
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app welcomeglobe`
Expected: PASS — all 8 tests green (`is_land_reads_the_bitmap`, `shade_char_spans_bright_to_dim`, `globe_cells_stay_within_its_box`, `globe_corners_are_outside_the_disc`, `globe_is_deterministic`, `globe_rotation_changes_the_frame`, `globe_shading_partitions_by_char_and_color`, `globe_visible_hemisphere_has_land_and_sea`).

- [ ] **Step 5: Partial gate (full workspace gate deferred to Task 2)**

Run: `cargo test -p crew-app welcomeglobe` (already green) and `cargo fmt --check -- crates/crew-app/src/welcomeglobe.rs crates/crew-app/src/main.rs`.
Expected: clean. Do **not** run the workspace-wide `cargo check` zero-warnings gate yet — `globe`, `GLOBE_W`, `GLOBE_H`, `GLOBE_MIN_W`, `GLOBE_MIN_H` are legitimately unused outside `#[cfg(test)]` until Task 2 wires `welcome.rs` to call them, so `cargo check -p crew-app` will show `dead_code` warnings at this point. That's expected and resolved in Task 2 Step 5.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/welcomeglobe.rs crates/crew-app/src/main.rs
git commit -m "feat(welcome): add the procedural ASCII globe renderer (welcomeglobe)"
```

---

### Task 2: Rewire `welcome.rs` onto the globe; delete the figlet banner and `welcomeart.rs`

**Files:**
- Modify: `crates/crew-app/src/welcome.rs` (full rewrite of the non-test section; test module replaced)
- Modify: `crates/crew-app/src/main.rs` (remove `mod welcomeart;`, line 123 as of Task 1's edit)
- Delete: `crates/crew-app/src/welcomeart.rs`

**Interfaces:**
- Consumes: `welcomeglobe::{globe, GLOBE_W, GLOBE_H, GLOBE_MIN_W, GLOBE_MIN_H}` from Task 1.
- Produces: `welcome_cells_animated(cols: u16, rows: u16, tick: u64) -> Vec<CellView>` — signature unchanged, still called from `render.rs` line 135 exactly as today (no caller-side changes needed). `pub const ANIM_DIV: u64` and `pub fn anim_should_redraw(tick: u64) -> bool` also unchanged (still consumed by `poll.rs` line 203 and `render.rs` line 133).

- [ ] **Step 1: Write the failing tests**

Replace the entire `#[cfg(test)] mod tests { ... }` block at the bottom of `crates/crew-app/src/welcome.rs` (currently lines 128-200) with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_cells_in_bounds() {
        let cells = welcome_cells_animated(80, 24, 7);
        assert!(!cells.is_empty());
        assert!(
            cells.iter().all(|c| c.col < 80 && c.row < 24),
            "cell out of 80×24 bounds"
        );
    }

    #[test]
    fn hint_present() {
        let cells = welcome_cells_animated(80, 24, 0);
        let hint_fg = crew_theme::theme().hint_fg;
        assert!(
            cells.iter().any(|c| c.fg == hint_fg),
            "no hint_fg cells in welcome output"
        );
    }

    #[test]
    fn version_stamp_present() {
        let cells = welcome_cells_animated(80, 24, 0);
        let dim = crew_theme::theme().dim;
        assert!(
            cells
                .iter()
                .any(|c| c.c == 'v' && c.row == 23 && c.fg == dim),
            "no version stamp on bottom row"
        );
    }

    #[test]
    fn tiny_size_no_panic_and_in_bounds() {
        let cells = welcome_cells_animated(2, 1, 0);
        assert!(cells.iter().all(|c| c.col < 2 && c.row < 1));
    }

    #[test]
    fn empty_screen_produces_cells() {
        assert!(!welcome_cells_animated(80, 24, 0).is_empty());
    }

    #[test]
    fn anim_redraws_one_in_every_anim_div_ticks() {
        let redraws = (0..ANIM_DIV * 4).filter(|&t| anim_should_redraw(t)).count();
        assert_eq!(redraws as u64, 4, "one redraw per ANIM_DIV ticks");
        assert!(anim_should_redraw(0) && anim_should_redraw(ANIM_DIV));
        assert!(!anim_should_redraw(1));
    }

    #[test]
    fn globe_width_picks_the_default_size_when_roomy() {
        assert_eq!(globe_width(90, 30), Some(44));
    }

    #[test]
    fn globe_width_scales_down_to_fit_the_rows() {
        // Default 44x22 needs rows > 25 (22 + 3); at rows=24 it steps down to 40x20.
        assert_eq!(globe_width(90, 24), Some(40));
    }

    #[test]
    fn globe_width_falls_back_when_nothing_fits() {
        assert_eq!(globe_width(10, 24), None, "too narrow for even the min width");
        assert_eq!(globe_width(90, 10), None, "too short for even the min height");
    }

    #[test]
    fn globe_sits_above_tagline_and_hint() {
        let cells = welcome_cells_animated(80, 30, 0);
        let t = crew_theme::theme();
        let globe_max_row = cells
            .iter()
            .filter(|c| c.fg == t.ink || c.fg == t.text_muted)
            .map(|c| c.row)
            .max()
            .expect("expected globe cells");
        let hint_min_row = cells
            .iter()
            .filter(|c| c.fg == t.hint_fg)
            .map(|c| c.row)
            .min()
            .expect("expected tagline/hint cells");
        assert!(globe_max_row < hint_min_row, "globe rows must sit above the tagline/hint");
    }

    #[test]
    fn welcome_animates_over_time() {
        let a = welcome_cells_animated(80, 30, 0);
        let b = welcome_cells_animated(80, 30, 20);
        let chars = |v: &[CellView]| v.iter().map(|c| (c.col, c.row, c.c, c.fg)).collect::<Vec<_>>();
        assert_ne!(chars(&a), chars(&b), "the globe frame must change over time");
    }
}
```

(This deletes `banner_cells_in_bounds` → renamed `welcome_cells_in_bounds`; deletes `banner_lines_equal_width` (banner is gone) and `shimmer_changes_over_time` (`col_style` is gone); adds `globe_width_*`, `globe_sits_above_tagline_and_hint`, `welcome_animates_over_time`.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app welcome::`
Expected: compile FAIL — `globe_width` not found in this scope (production code hasn't been rewired yet; `BANNER`, `col_style`, and the old `welcome_cells_animated` are all still present and still self-consistent at this point).

- [ ] **Step 3: Implement**

In `crates/crew-app/src/main.rs`, remove the now-stale `mod welcomeart;` line (added back in Task 1's edit context — `mod welcomeglobe;` from Task 1 stays):

```rust
mod welcome;
mod welcomeglobe;
mod windowtitle;
```

Delete the file:

```bash
git rm crates/crew-app/src/welcomeart.rs
```

Replace everything in `crates/crew-app/src/welcome.rs` above the `#[cfg(test)]` module (i.e. lines 1-126 of the pre-existing file) with:

```rust
//! The empty-screen welcome: a procedurally rendered, smoothly rotating
//! ASCII globe centred on the canvas, with a tagline + keyboard hint below
//! it and a version stamp in the corner.
use crew_render::CellView;

const TAGLINE: &str = "fast terminals. clean flow.";
const HINT: &str = "Cmd+T  new shell    ·    /  commands";
/// Poll ticks per rendered frame; idle animation runs at ~20 fps.
pub const ANIM_DIV: u64 = 3;

/// Whether this poll `tick` should redraw the welcome screen.
pub fn anim_should_redraw(tick: u64) -> bool {
    tick.is_multiple_of(ANIM_DIV)
}

/// Push every character of `s` as cells starting at `(col, row)`.
// rustfmt::skip keeps the CellView struct literal on one line.
#[rustfmt::skip]
fn push_str(cells: &mut Vec<CellView>, row: u16, col: u16, s: &str, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    for (i, ch) in s.chars().enumerate() {
        cells.push(CellView { col: col + i as u16, row, c: ch, fg, bg, bold: false, italic: false });
    }
}

/// Largest even globe width `w` (rendered at height `w/2`) such that the
/// globe + blank row + tagline + hint stack (`h + 3` rows) centres within
/// `rows`, and `w` (plus a 2-col margin) fits within `cols` — capped at
/// `welcomeglobe::GLOBE_W`, floored at `welcomeglobe::GLOBE_MIN_W`. `None`
/// when nothing fits — the caller falls back to the single-line banner.
fn globe_width(cols: u16, rows: u16) -> Option<u16> {
    let max_w = cols.saturating_sub(2).min(crate::welcomeglobe::GLOBE_W);
    let mut w = max_w - max_w % 2;
    while w >= crate::welcomeglobe::GLOBE_MIN_W {
        if w / 2 + 3 < rows {
            return Some(w);
        }
        w -= 2;
    }
    None
}

/// Render one animation frame: the rotating globe centred, tagline + hint
/// below it, version stamp bottom-right. Falls back to a spaced single-line
/// "CREW" when nothing globe-sized fits. All cells stay within `cols × rows`.
// rustfmt::skip preserves compact inline struct literals.
#[rustfmt::skip]
pub fn welcome_cells_animated(cols: u16, rows: u16, tick: u64) -> Vec<CellView> {
    if cols == 0 || rows == 0 { return Vec::new(); }
    let mut cells = Vec::new();
    let t = crew_theme::theme();
    let bg = t.page_bg;

    if let Some(w) = globe_width(cols, rows) {
        let h = w / 2;
        let top = (rows - (h + 3)) / 2;
        let left = (cols - w) / 2;
        let phase = tick as f32 * 0.05;
        crate::welcomeglobe::globe(&mut cells, top, left, w, h, phase, t.ink, t.text_muted, bg);

        let tl_row = top + h + 1;
        let tl_w = TAGLINE.chars().count() as u16;
        if tl_row < rows && tl_w < cols {
            push_str(&mut cells, tl_row, (cols - tl_w) / 2, TAGLINE, t.hint_fg, bg);
        }
        let hint_row = tl_row + 1;
        let hint_w = HINT.chars().count() as u16;
        if hint_row < rows && hint_w < cols {
            push_str(&mut cells, hint_row, (cols - hint_w) / 2, HINT, t.hint_fg, bg);
        }
    } else {
        // Fallback: spaced single-line "CREW" — same layout math as the old
        // figlet-era fallback, minus the deleted per-column shimmer (static ink).
        let letters: Vec<char> = "CREW".chars().collect();
        let span = (letters.len() as u16 - 1) * 2 + 1;
        if span < cols {
            let row   = rows / 2;
            let start = (cols - span) / 2;
            for (i, &ch) in letters.iter().enumerate() {
                cells.push(CellView { col: start + i as u16 * 2, row, c: ch, fg: t.ink, bg, bold: true, italic: false });
            }
            let hint_w   = HINT.chars().count() as u16;
            let hint_row = row + 2;
            if hint_w < cols && hint_row < rows {
                push_str(&mut cells, hint_row, (cols - hint_w) / 2, HINT, t.hint_fg, bg);
            }
        }
    }

    // Version stamp bottom-right.
    let ver = concat!("v", env!("CARGO_PKG_VERSION"));
    let vw = ver.chars().count() as u16;
    if vw + 1 < cols {
        push_str(&mut cells, rows - 1, cols - vw - 1, ver, t.dim, bg);
    }
    cells
}
```

(The `#[cfg(test)] mod tests { ... }` block from Step 1 stays below this, unchanged.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app welcome::`
Expected: PASS — all 11 tests green, including the 3 `globe_width_*` tests and the 2 new integration tests.

Also run: `cargo test -p crew-app welcomeglobe` — expected: still PASS (Task 1's 8 tests untouched).

- [ ] **Step 5: Full gate**

Run: `cargo check --workspace 2>&1 | grep -c warning` → expected `0` (this is the real gate: `welcomeglobe`'s pub surface is now consumed by `welcome.rs`, and `welcomeart.rs` — along with its own now-orphaned `SCENE_W`/`SCENE_H`/`scene` — is deleted, so nothing is left dangling).
Run: `cargo fmt --check`.
Run: `cargo test -p crew-app` (full package — every existing suite must stay green, not just `welcome`/`welcomeglobe`).
Run: `cargo build -p crew-app --bin crew` to confirm the binary still links (the welcome screen is on the app's empty-pane path).
Expected: all clean.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-app/src/welcome.rs crates/crew-app/src/main.rs
git status  # confirm crates/crew-app/src/welcomeart.rs shows as deleted
git commit -m "feat(welcome): rotate in the ASCII globe — delete the figlet banner and dev-figure scene"
```

---

## Self-Review Notes

- **Spec coverage:** pure/deterministic `globe()` (Task 1, `globe_is_deterministic`); theme-driven colors only (Task 2 passes `t.ink`/`t.text_muted`, never a literal RGB, into `globe()`); ~20 fps via unchanged `ANIM_DIV`/`phase = tick * 0.05` (Task 2, `welcome_cells_animated`); default 44×22 with ≥16×8 scale-down and fallback (Task 1 `GLOBE_W`/`GLOBE_MIN_W` consts + Task 2 `globe_width`); cell aspect 2:1 (Task 1 `globe()`'s `hw`/`hh` normalization, documented in its doc comment); continents recognizable and both land+sea visible at phase 0 (Task 1 bitmap + `globe_visible_hemisphere_has_land_and_sea`); banner/shimmer/dev-figure deletion + `welcomeart.rs` removal (Task 2 Step 3); stack ordering globe-above-tagline (Task 2, `globe_sits_above_tagline_and_hint`).

- **Resolved ambiguity — "kept verbatim" vs. "`col_style` shimmer... deleted":** The spec's Rendering section says the small-pane fallback is "the existing spaced single-line CREW branch (kept verbatim)", but its Layout section is explicit that "`col_style` shimmer... [is] deleted" alongside the banner constants. Read literally, both cannot hold — the current fallback code calls `col_style` for its per-letter color. I resolved this in favor of the more specific, explicit instruction (`col_style` deletion) and read "kept verbatim" as referring to the fallback's *layout math* (letter spacing, centering, hint placement), not its exact bytes: Task 2's fallback branch keeps the identical `letters`/`span`/`start`/`hint_row` arithmetic, but renders the letters in a static `t.ink` (bold) instead of calling the now-deleted `col_style`. This is the smallest change that satisfies both the "banner + shimmer all go" goal statement and the "kept verbatim" layout intent.

- **Build-order note on the zero-warnings gate (not a spec ambiguity, a sequencing fact verified empirically while planning):** A `pub fn`/`pub const` in a `bin` crate genuinely triggers `dead_code` warnings under plain `cargo check` when nothing outside `#[cfg(test)]` calls it yet, even though `cargo test` itself stays clean (test code counts as a use). I confirmed this against a scratch crate before writing the plan. Task 1 therefore gates on `cargo test` only; the workspace-wide zero-warnings assertion is Task 2 Step 5, once `welcome.rs` actually calls `globe()` and reads `GLOBE_W`/`GLOBE_H`/`GLOBE_MIN_W`/`GLOBE_MIN_H`.

- **Bitmap fidelity:** The 48×24 continent bitmap was derived by hand from real lat/lon bounding boxes for each landmass (7.5°-per-cell resolution), rendered to an ASCII grid, and round-trip-verified (hex → bits → ASCII) to confirm the packed `u64` rows decode back to the intended silhouette before being written into the plan. `is_land(10, 10)` (North America) and `is_land(10, 20)` (mid-Atlantic gap) in Task 1's test suite are concrete, checked coordinates into that bitmap, not placeholders.

- **Math sanity-checked, not just asserted:** Before finalizing, I compiled the exact `welcomeglobe.rs` implementation (bitmap, `is_land`, `shade_char`, `light_dir`, `globe`, and all 8 planned tests) in an isolated scratch crate and ran `cargo test` — all 8 pass against the real code in this plan, not hand-traced arithmetic. I also verified `globe_width`'s arithmetic (`globe_width(90,30) == Some(44)`, `globe_width(90,24) == Some(40)`, `globe_width(10,24) == None`, `globe_width(90,10) == None`) by hand-executing its loop for each case.

- **No caller-side changes needed:** `render.rs` (line 133-136) and `poll.rs` (line 199-218) already pass a `tick` into `welcome_cells_animated` that's pre-divided by `ANIM_DIV` and already drives redraws at ~20 fps — this plan reuses that value verbatim as `phase`'s input; neither file needs to change.

- **Out of scope confirmed:** no day/night terminator, no mouse interaction, no configurable speed/size, no flag to keep the old banner, no per-theme ocean colors beyond `land`/`sea` — matches the spec's YAGNI list, and nothing in this plan adds any of them.
