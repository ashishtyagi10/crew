//! Block elements and shades — U+2580–U+259F.
//!
//! These are the ones a font has the least excuse for: every character in the
//! range is *defined* as a fraction of the cell, and a font can only guess
//! what the cell is. Crew's meters, gauges, treemaps and heatmaps are built
//! out of them by the thousand, so an eighth-block that rounds to 3.4 pixels
//! and is then dilated is the difference between a bar with a straight end
//! and a bar with a grey haze on it.
//!
//! Every fraction here is rounded to whole pixels, which is also what makes
//! a row of `█` seamless: neighbouring cells cannot disagree about where the
//! edge is when there is no fractional edge to disagree about.
use super::Mask;

/// Coverage of the three shade characters, as a 0–1 fraction. A flat tint
/// rather than a dither: a dithered `▒` shimmers against the pixel grid the
/// moment anything scrolls, which is the opposite of crisp.
const SHADES: [f32; 3] = [0.25, 0.5, 0.75];

/// `(char, [upper_left, upper_right, lower_left, lower_right])`.
const QUADRANTS: &[(char, [bool; 4])] = &[
    ('\u{2596}', [false, false, true, false]),
    ('\u{2597}', [false, false, false, true]),
    ('\u{2598}', [true, false, false, false]),
    ('\u{2599}', [true, false, true, true]),
    ('\u{259A}', [true, false, false, true]),
    ('\u{259B}', [true, true, true, false]),
    ('\u{259C}', [true, true, false, true]),
    ('\u{259D}', [false, true, false, false]),
    ('\u{259E}', [false, true, true, false]),
    ('\u{259F}', [false, true, true, true]),
];

/// `n` eighths of `extent`, rounded to a whole pixel — never to none. On a
/// narrow cell several eighths round to the same width, which is what a
/// pixel grid can honestly say; rounding one of them to ZERO would draw a
/// blank where the character asks for a mark.
fn eighths(extent: u32, n: u32) -> f32 {
    (extent as f32 * n as f32 / 8.0).round().max(1.0)
}

/// Draw `c` if it is a block element or a shade.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let (w, h) = (m.w as f32, m.h as f32);
    let (hx, hy) = ((w / 2.0).round(), (h / 2.0).round());
    match c {
        // Full block, and the lower eighths that build every vertical bar.
        '\u{2581}'..='\u{2588}' => {
            let n = c as u32 - 0x2580;
            m.rect(0.0, h - eighths(m.h, n), w, h);
        }
        // Left eighths, counting DOWN from the full block: U+2589 is 7/8.
        '\u{2589}'..='\u{258F}' => {
            let n = 8 - (c as u32 - 0x2588);
            m.rect(0.0, 0.0, eighths(m.w, n), h);
        }
        '\u{2580}' => m.rect(0.0, 0.0, w, hy),
        '\u{2590}' => m.rect(hx, 0.0, w, h),
        '\u{2594}' => m.rect(0.0, 0.0, w, eighths(m.h, 1)),
        '\u{2595}' => m.rect(w - eighths(m.w, 1), 0.0, w, h),
        '\u{2591}'..='\u{2593}' => {
            m.rect_at(0.0, 0.0, w, h, SHADES[c as usize - 0x2591]);
        }
        '\u{2596}'..='\u{259F}' => {
            let Some(q) = QUADRANTS.iter().find(|(k, _)| *k == c).map(|(_, q)| *q) else {
                return false;
            };
            if q[0] {
                m.rect(0.0, 0.0, hx, hy);
            }
            if q[1] {
                m.rect(hx, 0.0, w, hy);
            }
            if q[2] {
                m.rect(0.0, hy, hx, h);
            }
            if q[3] {
                m.rect(hx, hy, w, h);
            }
        }
        _ => return false,
    }
    true
}
