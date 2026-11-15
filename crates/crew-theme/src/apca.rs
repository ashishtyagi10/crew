//! APCA lightness contrast (Lc), the contrast model of the WCAG 3 drafts.
//!
//! [`crate::contrast_ratio`] is WCAG 2's luminance ratio, and it is blind
//! to polarity: light text on a dark page measures as well as dark text on
//! a light one at a ratio a reader finds much harder. Every floor in crew is
//! a ratio, so a palette audit in Lc is how a dark page's colour is checked
//! against the light pages' (the code field, 2026-11-15). Lc runs 0 to
//! about 106 for dark on light and to about 108 for light on dark; the sign
//! is dropped here. Rough reading: 75 body text, 60 content text, 45 large
//! or bold text, 30 spot text, 15 the least a non-text mark can be.
//!
//! Constants: APCA-W3 0.0.98G-4g.

/// `|Lc|` of `text` on `bg`.
pub fn lc(text: (u8, u8, u8), bg: (u8, u8, u8)) -> f32 {
    let (yt, yb) = (y(text), y(bg));
    let s = if yb > yt {
        (yb.powf(0.56) - yt.powf(0.57)) * 1.14
    } else {
        (yb.powf(0.65) - yt.powf(0.62)) * 1.14
    };
    if s.abs() < 0.1 {
        return 0.0;
    }
    ((s.abs() - 0.027) * 100.0) as f32
}

/// Screen luminance, with APCA's soft clamp for the near-blacks.
fn y((r, g, b): (u8, u8, u8)) -> f64 {
    let f = |v: u8| (f64::from(v) / 255.0).powf(2.4);
    let y = 0.2126729 * f(r) + 0.7151522 * f(g) + 0.0721750 * f(b);
    match y < 0.022 {
        true => y + (0.022 - y).powf(1.414),
        false => y,
    }
}

#[cfg(test)]
#[path = "apca_tests.rs"]
mod tests;
