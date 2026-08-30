//! The marks that are a stroke rather than a shape — checks, crosses,
//! ballots and chevrons.
//!
//! Same argument as [`super::marks`], one step further along the census: with
//! the geometry drawn, what crew's chrome still borrowed from other faces was
//! `✗` and `⌘` from SF Mono, `❯` from Stelo, `☐` from a Nerd Font, and `⚑ ↵ ⇡`
//! from Menlo — while `✓`, right beside `✗` in every confirm prompt crew
//! draws, came from Lilex. A tick and a cross from two different typefaces is
//! the pair the eye is most likely to compare.
//!
//! These are drawn from one primitive — a capped line segment of the rules'
//! own weight — so they are the same colour, the same stroke and the same
//! optical size as every other mark and every rule around them. `⚑ ↵ ⇡ ⌘` are
//! left to the font: a flag, a return arrow and the command loop are drawings,
//! not constructions, and a hand-built one reads worse than a designed one.
use super::{light_thickness, Mask};

/// The mark box, as a fraction-of-the-cell mapper: `at(fx, fy)` turns
/// `0.0..=1.0` box coordinates into pixels. Everything below is written in
/// those, so the family scales as one.
fn mapper(m: &Mask) -> impl Fn(f32, f32) -> (f32, f32) {
    let s = m.w.min(m.h) as f32;
    let (cx, cy) = (m.w as f32 / 2.0, m.h as f32 / 2.0);
    move |fx: f32, fy: f32| (cx + (fx - 0.5) * s, cy + (fy - 0.5) * s)
}

/// A tick: a short fall into the corner and a long rise out of it.
fn check(m: &mut Mask, t: f32) {
    let at = mapper(m);
    m.stroke(at(0.16, 0.52), at(0.40, 0.78), t);
    m.stroke(at(0.40, 0.78), at(0.86, 0.20), t);
}

/// A cross: two diagonals of the box.
fn cross(m: &mut Mask, t: f32) {
    let at = mapper(m);
    m.stroke(at(0.18, 0.20), at(0.82, 0.80), t);
    m.stroke(at(0.82, 0.20), at(0.18, 0.80), t);
}

/// A chevron pointing right (`dir` +1) or left (−1).
fn chevron(m: &mut Mask, dir: f32, t: f32) {
    let at = mapper(m);
    let (a, b) = (0.5 - 0.22 * dir, 0.5 + 0.22 * dir);
    m.stroke(at(a, 0.16), at(b, 0.50), t);
    m.stroke(at(b, 0.50), at(a, 0.84), t);
}

/// The ballot box: a hollow square, larger than `□` — it is a control the eye
/// is meant to read a mark inside, not a bullet.
fn ballot(m: &mut Mask, t: f32) {
    let at = mapper(m);
    let (x0, y0) = at(0.08, 0.08);
    let (x1, y1) = at(0.92, 0.92);
    let (x0, y0, x1, y1) = (x0.round(), y0.round(), x1.round(), y1.round());
    m.rect(x0, y0, x1, y0 + t);
    m.rect(x0, y1 - t, x1, y1);
    m.rect(x0, y0, x0 + t, y1);
    m.rect(x1 - t, y0, x1, y1);
}

/// Draw `c` if it is one of the stroked marks.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let t = light_thickness(m.h) as f32;
    // The heavy variants are the same construction at half again the weight,
    // which is the relationship `━` keeps with `─`.
    let heavy = t * 1.6;
    match c {
        '\u{2713}' => check(m, t),
        '\u{2714}' => check(m, heavy),
        '\u{2717}' => cross(m, t),
        '\u{2718}' => cross(m, heavy),
        '\u{2610}' => ballot(m, t),
        '\u{2611}' => {
            ballot(m, t);
            check(m, t);
        }
        '\u{2612}' => {
            ballot(m, t);
            cross(m, t);
        }
        '\u{276F}' => chevron(m, 1.0, t),
        '\u{276E}' => chevron(m, -1.0, t),
        _ => return false,
    }
    true
}
