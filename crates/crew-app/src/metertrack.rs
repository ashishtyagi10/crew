//! What a meter's empty track is drawn in, so that it is still there.
//!
//! Two families of meter share this: the chat footer's rolling-window bars
//! (`summarymeter`) and the sidebar's system gauges (`gauges`). Both pick a
//! recessed colour and hand it to [`crate::plot::meter::capsule`], which lays
//! it down at [`crate::plot::meter::TRACK_ALPHA`] — and that alpha is half of
//! what the track ends up reading as. A colour chosen at 1.9:1 against the
//! page arrives at 1.3:1, which is why every one of these was measured
//! against the page as a COLOUR and passed while the pixels failed.
//!
//! The track earns its ink: it is what says how long the meter is, and a fill
//! is only a fraction of something you can see the whole of.
/// The least a track may read against the page once drawn. The dark pages
/// were nearest it already (1.46–1.57); the light ones were at 1.35.
pub(crate) const FLOOR: f32 = 1.6;

/// `c` as it will look once the capsule has laid it down at its alpha.
pub(crate) fn as_drawn(c: (u8, u8, u8), page: (u8, u8, u8)) -> (u8, u8, u8) {
    crate::anim::lerp_rgb(c, page, 1.0 - crate::plot::meter::TRACK_ALPHA)
}

/// Walk `from` toward `to` in twentieths until what is DRAWN clears
/// [`FLOOR`] against `page` — `to` itself is the end of the walk, and on
/// every theme in the set it clears the floor by a wide margin.
///
/// Lifted rather than dropped, the way `crew_theme::tagcolor` lifts a tag
/// into readability: the recessed colour is the intent, and the floor is the
/// least of it that survives.
pub(crate) fn lift(from: (u8, u8, u8), to: (u8, u8, u8), page: (u8, u8, u8)) -> (u8, u8, u8) {
    for k in 0..=20 {
        let c = crate::anim::lerp_rgb(from, to, k as f32 / 20.0);
        if crew_theme::contrast_ratio(as_drawn(c, page), page) >= FLOOR {
            return c;
        }
    }
    to
}

#[cfg(test)]
#[path = "metertrack_tests.rs"]
mod tests;
