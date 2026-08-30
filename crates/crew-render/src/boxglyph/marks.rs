//! The geometric marks — discs, rings, triangles, squares, diamonds.
//!
//! These are the ones crew's own chrome is written in: the activity dot on a
//! pane's border, the bullets in a list, the little arrow before a selected
//! row, the state chips. Asking a *font* for them turns out to be a lottery.
//! Shaped through crew's own stack on a stock Mac, with Lilex chosen, they
//! come back from **five different typefaces**:
//!
//! ```text
//! Lilex                     ○ ●
//! SF Mono                   ▶ ▸ ▴ ▾ ◂ ◀ ▲ ▼ ■ □
//! Stelo                     ◆ ◇ ▪ ▫
//! Apple Color Emoji         ⏺
//! ```
//!
//! Five faces means five ideas of how big a mark is, how heavy, and where it
//! sits above the baseline — in one row of chrome, next to each other. And
//! the emoji one is worse than inconsistent: a colour glyph carries its own
//! pixels, so `⏺` is the same red-and-white dot on every theme crew has,
//! ignoring the palette outright and arriving as a scaled bitmap.
//!
//! Every character here is *defined* as a shape rather than drawn as a
//! letterform, so crew draws them: one size relationship, one weight, the
//! theme's own colour, and the same on any machine and under any font.
use super::{light_thickness, Mask};

/// The mark box: a square of the cell's narrower side, centred on the same
/// centre the rules use — so a `●` in a list lines up with the `─` above it.
fn box_of(m: &Mask) -> (f32, f32, f32) {
    let s = m.w.min(m.h) as f32;
    (m.w as f32 / 2.0, m.h as f32 / 2.0, s)
}

/// Point inside the triangle `a b c` (winding-independent).
fn in_tri(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let side = |u: (f32, f32), v: (f32, f32)| (v.0 - u.0) * (p.1 - u.1) - (v.1 - u.1) * (p.0 - u.0);
    let (d1, d2, d3) = (side(a, b), side(b, c), side(c, a));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// A triangle pointing `dir` (0 up, 1 right, 2 down, 3 left), `frac` of the
/// mark box across. Isosceles and inscribed, so the four directions read as
/// one family rather than four unrelated wedges.
fn triangle(m: &mut Mask, dir: u8, frac: f32) {
    let (cx, cy, s) = box_of(m);
    let h = s * frac / 2.0;
    let tip = match dir {
        0 => (cx, cy - h),
        1 => (cx + h, cy),
        2 => (cx, cy + h),
        _ => (cx - h, cy),
    };
    let (a, b) = match dir {
        0 => ((cx - h, cy + h), (cx + h, cy + h)),
        1 => ((cx - h, cy - h), (cx - h, cy + h)),
        2 => ((cx - h, cy - h), (cx + h, cy - h)),
        _ => ((cx + h, cy - h), (cx + h, cy + h)),
    };
    m.sample(move |x, y| in_tri((x, y), tip, a, b));
}

/// A filled or hollow disc of `frac` of the mark box.
fn disc(m: &mut Mask, frac: f32, hollow: bool) {
    let (cx, cy, s) = box_of(m);
    let r = s * frac / 2.0;
    let inner = if hollow {
        (r - light_thickness(m.h) as f32).max(0.0)
    } else {
        0.0
    };
    m.sample(move |x, y| {
        let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        d <= r && d >= inner
    });
}

/// A filled or hollow diamond of `frac` of the mark box.
fn diamond(m: &mut Mask, frac: f32, hollow: bool) {
    let (cx, cy, s) = box_of(m);
    let r = s * frac / 2.0;
    // The hollow one keeps a stroke of the same weight the rules use, so a
    // `◇` beside a `│` is the same line.
    let inner = if hollow {
        (r - light_thickness(m.h) as f32 * 1.6).max(0.0)
    } else {
        0.0
    };
    m.sample(move |x, y| {
        let d = (x - cx).abs() + (y - cy).abs();
        d <= r && d >= inner
    });
}

/// A filled or hollow square of `frac` of the mark box — whole pixels, so it
/// has no fringe at all.
fn square(m: &mut Mask, frac: f32, hollow: bool) {
    let (cx, cy, s) = box_of(m);
    let h = (s * frac / 2.0).round().max(1.0);
    let (x0, y0) = ((cx - h).round(), (cy - h).round());
    let (x1, y1) = (x0 + 2.0 * h, y0 + 2.0 * h);
    if !hollow {
        m.rect(x0, y0, x1, y1);
        return;
    }
    let t = light_thickness(m.h) as f32;
    m.rect(x0, y0, x1, y0 + t);
    m.rect(x0, y1 - t, x1, y1);
    m.rect(x0, y0, x0 + t, y1);
    m.rect(x1 - t, y0, x1, y1);
}

/// Draw `c` if it is one of the geometric marks.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    match c {
        '\u{25CF}' => disc(m, 0.68, false),
        '\u{23FA}' => disc(m, 0.80, false),
        '\u{25CB}' => disc(m, 0.68, true),
        '\u{25B2}' => triangle(m, 0, 0.76),
        '\u{25BC}' => triangle(m, 2, 0.76),
        '\u{25C0}' => triangle(m, 3, 0.76),
        '\u{25B6}' => triangle(m, 1, 0.76),
        '\u{25B4}' => triangle(m, 0, 0.52),
        '\u{25BE}' => triangle(m, 2, 0.52),
        '\u{25C2}' => triangle(m, 3, 0.52),
        '\u{25B8}' => triangle(m, 1, 0.52),
        '\u{25A0}' => square(m, 0.72, false),
        '\u{25A1}' => square(m, 0.72, true),
        '\u{25AA}' => square(m, 0.46, false),
        '\u{25AB}' => square(m, 0.46, true),
        '\u{25C6}' => diamond(m, 0.92, false),
        '\u{25C7}' => diamond(m, 0.92, true),
        _ => return false,
    }
    true
}
