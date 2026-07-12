//! The empty-screen welcome: a procedurally rendered, smoothly rotating
//! ASCII globe centred on the canvas, with a tagline + keyboard hint below
//! it and a version stamp in the corner.
use crew_render::CellView;

use crate::welcomeglobe::{GLOBE_H, GLOBE_MIN_H, GLOBE_MIN_W, GLOBE_W};

const TAGLINE: &str = "fast terminals. clean flow.";
const HINT: &str = "Cmd+T  new shell    ·    /  commands";
/// Poll ticks per rendered frame; idle animation runs at ~20 fps.
pub const ANIM_DIV: u64 = 3;

/// Width-to-height ratio of the globe box (2:1, matching the terminal's
/// ~2:1 cell aspect) — derives `h` from `w` without hardcoding `/2`.
const ASPECT: u16 = GLOBE_W / GLOBE_H;

// Compile-time guard: GLOBE_MIN_H must keep tracking GLOBE_MIN_W's aspect
// ratio, so this file's `ASPECT`-based derivation never silently drifts
// from welcomeglobe.rs's floor.
const _: () = assert!(
    GLOBE_MIN_H == GLOBE_MIN_W / ASPECT,
    "GLOBE_MIN_H must track GLOBE_MIN_W's 2:1 aspect"
);

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
    let max_w = cols.saturating_sub(2).min(GLOBE_W);
    let mut w = max_w - max_w % 2;
    while w >= GLOBE_MIN_W {
        if w / ASPECT + 3 < rows {
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
        let h = w / ASPECT;
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
        assert_eq!(
            globe_width(10, 24),
            None,
            "too narrow for even the min width"
        );
        assert_eq!(
            globe_width(90, 10),
            None,
            "too short for even the min height"
        );
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
        assert!(
            globe_max_row < hint_min_row,
            "globe rows must sit above the tagline/hint"
        );
    }

    #[test]
    fn welcome_animates_over_time() {
        let a = welcome_cells_animated(80, 30, 0);
        let b = welcome_cells_animated(80, 30, 20);
        let chars = |v: &[CellView]| {
            v.iter()
                .map(|c| (c.col, c.row, c.c, c.fg))
                .collect::<Vec<_>>()
        };
        assert_ne!(
            chars(&a),
            chars(&b),
            "the globe frame must change over time"
        );
    }
}
