//! Capsule meters: a proportion drawn as a rounded bar.
//!
//! The dithered `▓`/`░` gauges these replace could only be eight cells long
//! and eight steps deep, so a budget crossing 6% moved nothing at all and a
//! meter at 12% and one at 24% drew the same first cell. A drawn capsule is
//! continuous — the fill lands where the number says, to a fraction of a cell.
use crate::plot::{sdf, Canvas};

/// How tall the capsule is inside its row, as a fraction of the row.
const HEIGHT: f32 = 0.30;

/// How solidly the empty track is laid down. Public because it is half of
/// what the track's colour ends up being: a caller choosing that colour
/// against the page has to blend at this alpha to know what it will read as
/// (see `crate::metertrack`).
///
/// **Solid when the OS asks for more contrast.** A groove at 55% alpha has a
/// ceiling: past a point no colour choice can clear the raised mark floor,
/// because the alpha is what is eating it. The mode that says "I cannot see
/// faint things" is exactly the mode where a track stops being faint.
pub fn track_alpha() -> f32 {
    match crew_theme::contrast::high_contrast() {
        true => 1.0,
        false => 0.55,
    }
}

/// Draw a meter filling `frac` of `(x, y, w)` — one text row tall, centred in
/// it. `shade` gives the fill's colour along its length (`0.0..=1.0`), so a
/// meter can walk a gradient; `track` is the unfilled remainder, which is
/// always drawn: a meter's length is half of what it says.
#[allow(clippy::too_many_arguments)] // a meter is a rect, a reading, and two colours
pub fn capsule(
    c: &mut Canvas,
    x: f32,
    y: f32,
    w: f32,
    row_h: f32,
    frac: f32,
    shade: impl Fn(f32) -> (u8, u8, u8),
    track: (u8, u8, u8),
) {
    let h = (row_h * HEIGHT).max(0.12);
    let top = y + (row_h - h) * 0.5;
    let r = h * 0.5;
    if w <= 0.0 {
        return;
    }
    // The track: the full length, so the eye has something to read the fill
    // against even at 0%.
    rounded(c, x, top, w, h, r, |_| track, track_alpha());

    let frac = frac.clamp(0.0, 1.0);
    if frac <= 0.0 {
        return;
    }
    // A fill shorter than its own end caps would draw as a lozenge floating
    // in the track; below that it is drawn as a dot at the left end, which is
    // what "barely started" looks like.
    let fw = (w * frac).max(h.min(w));
    rounded(c, x, top, fw, h, r, |t| shade(t * frac.max(0.001)), 1.0);
}

/// A rounded rectangle: the pill shape both the track and the fill are.
#[allow(clippy::too_many_arguments)] // a rect, a corner radius, a shade, an alpha
fn rounded(
    c: &mut Canvas,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    shade: impl Fn(f32) -> (u8, u8, u8),
    alpha: f32,
) {
    c.fill_sdf_shaded(
        (x, y, w, h),
        move |px, py| sdf::round_box((px, py), x, y, w, h, r),
        move |px, _| {
            let t = if w > 0.0 {
                ((px - x) / w).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (shade(t), alpha)
        },
    );
}

#[cfg(test)]
#[path = "meter_tests.rs"]
mod tests;
