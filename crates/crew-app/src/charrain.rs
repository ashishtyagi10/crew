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
/// Glyph alphabet — ASCII only (single-width, font-safe; no CJK, which has
/// advance-width hazards on some fonts). Letters and digits, and a few quiet
/// marks the trail may wear: the `$#%&\\{}` the field once rained read as a
/// corrupted terminal, not as light.
const GLYPHS: &[u8] = b"0123456789abcdefhkmnrsvxyz<>/=+:";
/// The leading run of [`GLYPHS`] a head may be: letters and digits only.
const HEADS: usize = 26;
/// Trail cells from this depth on are dots: the streak dissolves as it fades.
const DOTS_FROM: u16 = 3;
/// A column rests between streaks for up to this many cells' fall, so the
/// field is a few streaks at a time and mostly page.
const REST: u64 = 12;
/// Cells from the rect's edge over which the field fades out: the rain is
/// bounded by light, not by a ruled box around it.
const FADE: f32 = 3.0;
/// How much of its light a glyph ON the edge keeps.
const EDGE: f32 = 0.3;

/// How far the rain stays back from the field's centre, where the welcome
/// sets its name: the fraction of the box's width and height the calm spans
/// before the glyphs start to come back (they are fully back at `CALM_OUT`×
/// that). A plate of blanks the size of the word let streaks run right up to
/// its letters, and the name read as four more glyphs of the field.
const CALM_W: f32 = 0.17;
const CALM_H: f32 = 0.16;
const CALM_OUT: f32 = 1.8;

/// How much of its light a glyph at `(col, row)` keeps for being near the
/// centre: 0 inside the calm, easing to 1 at its outer edge.
fn calm(col: u16, row: u16, w: u16, h: u16) -> f32 {
    let dx = (col as f32 + 0.5 - w as f32 / 2.0) / (w as f32 * CALM_W).max(9.0);
    let dy = (row as f32 + 0.5 - h as f32 / 2.0) / (h as f32 * CALM_H).max(2.0);
    let k = (((dx * dx + dy * dy).sqrt() - 1.0) / (CALM_OUT - 1.0)).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
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
    let fall = tick / 2;   // how far heads have dropped
    let flick = tick / 6;  // glyph re-roll clock (slower than the fall)
    // The tail's last cells lean on into the page, so a streak fades out
    // rather than stopping at a muted wall.
    let tail = lerp_rgb(trail, bg, 0.45);
    for col in 0..w {
        let seed = hash(col as u64, 0x51);
        let delay = 1 + seed % 3; // columns take 1..=3 fall-ticks per cell dropped
        let period = h as u64 + TRAIL as u64 + h as u64 / 2 + seed % REST;
        let headrow = ((fall / delay + seed % period) % period) as i64;
        for d in 0..TRAIL {
            let r = headrow - d as i64;
            if r < 0 || r >= h as i64 { continue; }
            let bright = 1.0 - d as f32 / TRAIL as f32;
            let n = if d == 0 { HEADS } else { GLYPHS.len() } as u64;
            let gi = (hash(col as u64, (r as u64) ^ flick) % n) as usize;
            let c = if d >= DOTS_FROM { '\u{00b7}' } else { GLYPHS[gi] as char };
            let e = (col + 1).min(w - col).min(r as u16 + 1).min(h - r as u16) as f32;
            let fade = EDGE + (1.0 - EDGE) * ((e - 1.0) / FADE).clamp(0.0, 1.0);
            let fade = fade * calm(col, r as u16, w, h);
            if fade < 0.08 { continue; }
            cells.push(CellView {
                col: left + col, row: top + r as u16, c,
                fg: lerp_rgb(bg, lerp_rgb(tail, head, bright), fade), bg, bold: d == 0, italic: false,
                ..Default::default()
            });
        }
    }
}

#[cfg(test)]
#[path = "charrain_tests.rs"]
mod tests;
