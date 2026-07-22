//! A bounded "matrix rain" glyph field: pseudo-random characters falling in
//! columns within a given rect. Pure and deterministic in `tick` (no RNG), so
//! it renders identically for identical inputs — testable and resume-safe.
//! Give it a rect and it drops glyphs inside it and nowhere else (the page
//! shows through the gaps). Backs the welcome screen (replacing the old
//! globe); busy panes use a progress bar instead (see [`crate::chatprog`]).
use crew_render::CellView;

/// Default rain box: 64×16 cells — a wide, low 4:1 rectangle (~2:1 on screen
/// with ~2:1 cell aspect), framing the welcome nameplate.
pub const RAIN_W: u16 = 64;
pub const RAIN_H: u16 = RAIN_W / 4;
/// Smallest box still worth drawing; below this the welcome screen falls back
/// to its single-line banner.
pub const RAIN_MIN_W: u16 = 20;
pub const RAIN_MIN_H: u16 = RAIN_MIN_W / 4;

/// Trail length in cells behind each falling head.
const TRAIL: u16 = 6;
/// Glyph alphabet — ASCII/symbol only (single-width, font-safe; no CJK, which
/// has advance-width hazards on some fonts).
const GLYPHS: &[u8] = b"01<>[]{}()/\\|=+*-_#%&$?!:abcdefhkmnrsvyz3579";

/// One glyph from the rain alphabet for `seed` — lets other surfaces (the
/// smith splash's blinking box glyphs) flicker in the same character set.
pub(crate) fn glyph(seed: u64) -> char {
    GLYPHS[(hash(seed, 0xA7) % GLYPHS.len() as u64) as usize] as char
}

/// A fast integer hash (SplitMix-style) — the deterministic stand-in for RNG.
fn hash(a: u64, b: u64) -> u64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^ (x >> 33)
}

/// Linear RGB blend `a`→`b` at `t` in `[0,1]`.
fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

/// Render one rain frame into `cells`: a `w`×`h` box at `(top,left)`, advanced
/// by `tick`. Each column falls at its own pace, one cell at a time; the head
/// cell is brightest (`head`, bold), the trail fades toward `trail`. Only lit
/// cells are pushed.
#[rustfmt::skip]
#[allow(clippy::too_many_arguments)] // rect + tick + three colours, all independent
pub fn rain(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16,
            tick: u64, head: (u8,u8,u8), trail: (u8,u8,u8), bg: (u8,u8,u8)) {
    if w == 0 || h == 0 { return; }
    let period = h as u64 + TRAIL as u64;
    let fall = tick / 2;   // how far heads have dropped
    let flick = tick / 6;  // glyph re-roll clock (slower than the fall)
    for col in 0..w {
        let seed = hash(col as u64, 0x51);
        let delay = 1 + seed % 3; // columns take 1..=3 fall-ticks per cell dropped
        let headrow = ((fall / delay + seed % period) % period) as i64;
        for d in 0..TRAIL {
            let r = headrow - d as i64;
            if r < 0 || r >= h as i64 { continue; }
            let bright = 1.0 - d as f32 / TRAIL as f32;
            let gi = (hash(col as u64, (r as u64) ^ flick) % GLYPHS.len() as u64) as usize;
            cells.push(CellView {
                col: left + col, row: top + r as u16, c: GLYPHS[gi] as char,
                fg: lerp_rgb(trail, head, bright), bg, bold: d == 0, italic: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u16, h: u16, tick: u64) -> Vec<CellView> {
        let mut cells = Vec::new();
        rain(
            &mut cells,
            3,
            5,
            w,
            h,
            tick,
            (0, 255, 0),
            (0, 60, 0),
            (0, 0, 0),
        );
        cells
    }

    #[test]
    fn stays_inside_the_box_and_is_non_empty() {
        let cells = frame(RAIN_W, RAIN_H, 7);
        assert!(!cells.is_empty(), "default-size rain should emit drops");
        assert!(
            cells
                .iter()
                .all(|c| c.col >= 5 && c.col < 5 + RAIN_W && c.row >= 3 && c.row < 3 + RAIN_H),
            "every cell must stay within the given rect"
        );
    }

    #[test]
    fn head_cells_are_bold_and_brightest() {
        let cells = frame(RAIN_W, RAIN_H, 7);
        let heads: Vec<_> = cells.iter().filter(|c| c.bold).collect();
        assert!(!heads.is_empty(), "each active column has a bold head");
        assert!(
            heads.iter().all(|c| c.fg == (0, 255, 0)),
            "head is head colour"
        );
    }

    #[test]
    fn deterministic_in_tick_but_animates() {
        let same = |t| {
            frame(RAIN_W, RAIN_H, t)
                .iter()
                .map(|c| (c.col, c.row, c.c, c.fg))
                .collect::<Vec<_>>()
        };
        assert_eq!(same(7), same(7), "identical tick → identical frame");
        assert_ne!(same(0), same(20), "frames must change over time");
    }

    #[test]
    fn zero_size_emits_nothing() {
        assert!(frame(0, 10, 3).is_empty() && frame(10, 0, 3).is_empty());
    }
}
