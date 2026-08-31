//! Colour-space conversion at the GPU boundary. Theme colours are sRGB u8
//! triples; an sRGB render target expects LINEAR values from the shader and
//! encodes them on write. Feeding raw `c/255` fractions to such a target
//! double-encodes — every theme renders washed-out (a near-black (8,8,8) page
//! came out ≈(52,52,52)). Convert once, here, per target format.

/// One sRGB u8 channel → linear f32 (IEC 61966-2-1).
pub fn srgb_channel_to_linear(c: u8) -> f32 {
    let x = c as f32 / 255.0;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// An sRGB u8 triple as the `[r, g, b, a]` a render target of `srgb`-ness
/// expects: linear for sRGB targets, raw fractions for linear targets.
pub fn target_rgba(c: (u8, u8, u8), alpha: f32, srgb: bool) -> [f32; 4] {
    if srgb {
        [
            srgb_channel_to_linear(c.0),
            srgb_channel_to_linear(c.1),
            srgb_channel_to_linear(c.2),
            alpha,
        ]
    } else {
        [
            c.0 as f32 / 255.0,
            c.1 as f32 / 255.0,
            c.2 as f32 / 255.0,
            alpha,
        ]
    }
}

#[cfg(test)]
#[path = "color_tests.rs"]
mod tests;
