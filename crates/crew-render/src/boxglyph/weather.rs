//! The weather marks — sun, cloud, umbrella, snowflake, bolt.
//!
//! The nav's WEATHER card and the clock's strip lead with one of these, and
//! the embedded face has none of them: each came from whatever fallback face
//! the system found, at that face's idea of a symbol's size. The cloud
//! arrived as a flat hump a third of a cell tall, the umbrella as a speck
//! half a cell wide — the card's one picture, and the smallest thing on it.
//! Drawn, they fill the cell's width, sit on the text's centre line, and take
//! the theme's colour like every other mark (`marks`).
use super::{light_thickness, Mask};

/// The weather box: the cell's width across, and as tall as a capital —
/// not the square of the narrower side `marks` uses. A mark in a square as
/// wide as a cell stood a third the height of the `24°C` beside it; these are
/// pictures in a line of text, so they take the text's height. `(cx, cy)` is
/// the cell's centre, `sx`/`sy` the box's width and height.
#[derive(Clone, Copy)]
struct Frame {
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
}

impl Frame {
    fn of(m: &Mask) -> Self {
        let (w, h) = (m.w as f32, m.h as f32);
        Frame {
            cx: w / 2.0,
            cy: h / 2.0,
            sx: w,
            sy: (h * 0.5).min(w * 1.3),
        }
    }

    /// `(u, v)` in the box's unit square, centred: `-0.5..0.5` each way.
    fn at(&self, u: f32, v: f32) -> (f32, f32) {
        (self.cx + u * self.sx, self.cy + v * self.sy)
    }

    /// Whether pixel point `(x, y)` is inside the ellipse at unit `(u, v)`
    /// with unit radii `(ru, rv)`.
    fn in_ellipse(&self, (x, y): (f32, f32), (u, v): (f32, f32), (ru, rv): (f32, f32)) -> bool {
        let (ox, oy) = self.at(u, v);
        ((x - ox) / (ru * self.sx)).powi(2) + ((y - oy) / (rv * self.sy)).powi(2) <= 1.0
    }
}

/// ☀ — a disc and eight short rays, kept round: the radius is the box's
/// narrower side.
fn sun(m: &mut Mask) {
    let f = Frame::of(m);
    let t = light_thickness(m.h) as f32;
    let r = f.sx.min(f.sy);
    let (cx, cy) = (f.cx, f.cy);
    m.sample(move |x, y| (x - cx).powi(2) + (y - cy).powi(2) <= (r * 0.22).powi(2));
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::FRAC_PI_4;
        let (dx, dy) = (a.cos(), a.sin());
        m.stroke(
            (cx + dx * r * 0.34, cy + dy * r * 0.34),
            (cx + dx * r * 0.49, cy + dy * r * 0.49),
            t,
        );
    }
}

/// ☁ — three puffs on a flat base.
fn cloud(m: &mut Mask) {
    let f = Frame::of(m);
    m.sample(move |x, y| {
        let p = (x, y);
        let (bx0, by0) = f.at(-0.26, 0.04);
        let (bx1, by1) = f.at(0.28, 0.34);
        f.in_ellipse(p, (-0.26, 0.12), (0.22, 0.22))
            || f.in_ellipse(p, (0.04, -0.08), (0.3, 0.38))
            || f.in_ellipse(p, (0.28, 0.14), (0.2, 0.2))
            || (x >= bx0 && x <= bx1 && y >= by0 && y <= by1)
    });
}

/// ☂ — a domed canopy and a hooked handle.
fn umbrella(m: &mut Mask) {
    let f = Frame::of(m);
    let t = light_thickness(m.h) as f32;
    let (_, rim) = f.at(0.0, -0.02);
    m.sample(move |x, y| y <= rim && f.in_ellipse((x, y), (0.0, -0.02), (0.48, 0.46)));
    let top = f.at(0.0, -0.02);
    let foot = f.at(0.0, 0.42);
    let bend = f.at(-0.12, 0.5);
    let tip = f.at(-0.24, 0.4);
    m.stroke(top, foot, t);
    m.stroke(foot, bend, t);
    m.stroke(bend, tip, t);
}

/// ❄ — three strokes through the centre, sixty degrees apart.
fn snowflake(m: &mut Mask) {
    let f = Frame::of(m);
    let t = light_thickness(m.h) as f32;
    for k in 0..3 {
        let a = std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 3.0;
        let (u, v) = (a.cos() * 0.46, a.sin() * 0.46);
        m.stroke(f.at(-u, -v), f.at(u, v), t);
    }
}

/// ⚡ — a zigzag bolt, taller than it is wide.
fn bolt(m: &mut Mask) {
    let f = Frame::of(m);
    let pts = [
        f.at(0.12, -0.5),
        f.at(-0.3, 0.06),
        f.at(-0.02, 0.06),
        f.at(-0.12, 0.5),
        f.at(0.3, -0.06),
        f.at(0.02, -0.06),
    ];
    m.sample(move |x, y| {
        // Even-odd crossings: the bolt is one simple polygon.
        let mut inside = false;
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0) {
                inside = !inside;
            }
        }
        inside
    });
}

/// Draw `c` if it is one of the weather marks.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    match c {
        '\u{2600}' => sun(m),
        '\u{2601}' => cloud(m),
        '\u{2602}' => umbrella(m),
        '\u{2744}' => snowflake(m),
        '\u{26A1}' => bolt(m),
        _ => return false,
    }
    true
}

#[cfg(test)]
#[path = "weather_tests.rs"]
mod tests;
