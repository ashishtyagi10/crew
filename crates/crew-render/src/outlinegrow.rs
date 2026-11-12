//! Growing a glyph's outline, with the glyph's winding read correctly.
//!
//! swash has this (`Outline::embolden`, FreeType's algorithm), and it is
//! wrong for any glyph made of more than one contour. Which side of an edge
//! is "out" depends on the way the font winds its contours, and swash reads
//! that off a signed area taken over every point of the glyph as ONE
//! polygon — so the jump from the stem of an `i` to its dot, an edge the
//! glyph does not have, is counted with the rest. Where the jump outweighs
//! the shape the sign flips, and the glyph is thinned by the amount it
//! should have gained.
//!
//! Reported (2026-11-11): "letter i rendering in different color". It was
//! the same colour with a third of its ink gone: in SF Mono, JetBrains Mono,
//! PT Mono and Operator Mono the `i` came out at 55–75% of its ungrown
//! weight beside neighbours at 130%, and thin faces lost `p`, `b` and `%`
//! almost whole.
//!
//! This is the same algorithm with the area summed contour by contour, each
//! closed on itself. A counter winds against its glyph and takes its share
//! back out, as it should: the sign is the outer shape's.
use swash::zeno::{Point, Verb};

/// Move every edge of the outline out by `strength` pixels, then the whole
/// outline up and right by the same — a stroke comes back two strengths
/// wider with its left and bottom edges where they were.
pub(crate) fn grow(points: &mut [Point], verbs: &[Verb], strength: f32) {
    let contours = contours(verbs);
    let area: f32 = contours.iter().map(|c| area(&points[c.clone()])).sum();
    for c in contours {
        embolden(&mut points[c], area > 0.0, strength);
    }
}

/// The point range of each contour, in order.
fn contours(verbs: &[Verb]) -> Vec<std::ops::Range<usize>> {
    let (mut out, mut start, mut pos) = (Vec::new(), 0, 0);
    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                out.push(start..pos);
                start = pos;
                pos += 1;
            }
            Verb::LineTo => pos += 1,
            Verb::QuadTo => pos += 2,
            Verb::CurveTo => pos += 3,
            Verb::Close => {}
        }
    }
    out.push(start..pos);
    out.retain(|c| !c.is_empty());
    out
}

/// Twice the signed area of one contour's control polygon.
fn area(points: &[Point]) -> f32 {
    let Some(mut prev) = points.last().copied() else {
        return 0.0;
    };
    let mut area = 0.0;
    for cur in points {
        area += (cur.y - prev.y) * (cur.x + prev.x);
        prev = *cur;
    }
    area
}

/// FreeType's `FT_Outline_EmboldenXY` on one contour, as swash ports it:
/// each point moves along the bisector of the edges meeting at it, no
/// further than the shorter of the two allows.
fn embolden(points: &mut [Point], positive: bool, strength: f32) {
    let Some(last) = points.len().checked_sub(1) else {
        return;
    };
    let next = |n: usize| if n < last { n + 1 } else { 0 };
    let (mut i, mut j, mut k) = (last, 0, usize::MAX);
    let (mut in_, mut in_len) = (Point::ZERO, 0.0f32);
    let (mut anchor, mut anchor_len) = (Point::ZERO, 0.0f32);
    while j != i && i != k {
        let (out, out_len) = if j != k {
            let out = points[j] - points[i];
            let len = out.length();
            if len == 0.0 {
                j = next(j);
                continue;
            }
            (Point::new(out.x / len, out.y / len), len)
        } else {
            (anchor, anchor_len)
        };
        if in_len != 0.0 {
            if k == usize::MAX {
                k = i;
                anchor = in_;
                anchor_len = in_len;
            }
            let shift = shift(in_, out, in_len.min(out_len), positive, strength);
            while i != j {
                points[i].x += strength + shift.x;
                points[i].y += strength + shift.y;
                i = next(i);
            }
        } else {
            i = j;
        }
        in_ = out;
        in_len = out_len;
        j = next(j);
    }
}

/// How far a corner moves, given the unit directions into and out of it and
/// the length of the shorter edge. A corner sharper than 160° stays put.
fn shift(in_: Point, out: Point, shorter: f32, positive: bool, strength: f32) -> Point {
    let dot = in_.x * out.x + in_.y * out.y;
    if dot <= -0.9396 {
        return Point::ZERO;
    }
    let d = dot + 1.0;
    let sign = if positive { 1.0 } else { -1.0 };
    let (sx, sy) = (sign * (in_.y + out.y), -sign * (in_.x + out.x));
    let q = sign * (out.x * in_.y - out.y * in_.x);
    let scale = match strength * q <= shorter * d {
        true => strength / d,
        false => shorter / q,
    };
    Point::new(sx * scale, sy * scale)
}

#[cfg(test)]
#[path = "outlinegrow_tests.rs"]
mod tests;
