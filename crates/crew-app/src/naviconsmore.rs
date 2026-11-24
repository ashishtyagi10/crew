//! The Inbox and Far icons — split from [`crate::navicons`] for the line cap. Same manner:
//! outline shapes as signed distances, in a box of half-size `u` around `at`, stroke half-width
//! `w`.
use crate::plot::sdf;

/// Point `(x, y)` of the icon's -1..1 box, in canvas units.
fn at_box(at: (f32, f32), u: f32, x: f32, y: f32) -> (f32, f32) {
    (at.0 + x * u, at.1 + y * u)
}

fn outlined_box(p: (f32, f32), at: (f32, f32), u: f32, w: f32, b: (f32, f32, f32, f32)) -> f32 {
    let (x0, y0) = at_box(at, u, b.0, b.1);
    sdf::round_box(p, x0, y0, b.2 * u, b.3 * u, 0.16 * u).abs() - w
}

fn line(p: (f32, f32), at: (f32, f32), u: f32, w: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    sdf::capsule(p, at_box(at, u, a.0, a.1), at_box(at, u, b.0, b.1), w)
}

/// An envelope: its body, and the flap's V down to the middle.
pub(crate) fn inbox(p: (f32, f32), at: (f32, f32), u: f32, w: f32) -> f32 {
    let body = outlined_box(p, at, u, w, (-0.95, -0.65, 1.9, 1.3));
    let left = line(p, at, u, w, (-0.85, -0.55), (0.0, 0.12));
    let right = line(p, at, u, w, (0.0, 0.12), (0.85, -0.55));
    body.min(left).min(right)
}

/// Far's two panels side by side, each with a listing's lines in it — a commander, where the
/// Panes icon is one window split.
pub(crate) fn far(p: (f32, f32), at: (f32, f32), u: f32, w: f32) -> f32 {
    let mut d = f32::MAX;
    for x in [-0.95, 0.1] {
        d = d.min(outlined_box(p, at, u, w, (x, -0.8, 0.85, 1.6)));
        for y in [-0.35, 0.05, 0.45] {
            d = d.min(line(p, at, u, w, (x + 0.22, y), (x + 0.63, y)));
        }
    }
    d
}
