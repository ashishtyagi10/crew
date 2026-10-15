//! What a meter's empty track is drawn in, so that it is still there.
//!
//! Two families of meter share this: the chat footer's rolling-window bars
//! (`summarymeter`) and the sidebar's system gauges (`gauges`). Both pick a
//! recessed colour and hand it to [`crate::plot::meter::capsule`], which lays
//! it down at [`crate::plot::meter::track_alpha`] — and that alpha is half of
//! what the track ends up reading as. A colour chosen at 1.9:1 against the
//! page arrives at 1.3:1, which is why every one of these was measured
//! against the page as a COLOUR and passed while the pixels failed.
//!
//! The track earns its ink: it is what says how long the meter is, and a fill
//! is only a fraction of something you can see the whole of.
/// The least a track may read against the page once drawn. The dark pages
/// were nearest it already (1.46–1.57); the light ones were at 1.35.
const QUIET_FLOOR: f32 = 1.6;

/// …and what it becomes when the OS asks for more contrast: a groove is a
/// mark then, on the same floor every other mark answers to. A track is the
/// quietest thing crew draws on purpose, which is exactly the kind of thing
/// "increase contrast" exists to reach.
pub(crate) fn floor() -> f32 {
    match crew_theme::contrast::high_contrast() {
        true => crew_theme::contrast::mark_floor(),
        false => QUIET_FLOOR,
    }
}

/// `c` as it will look once the capsule has laid it down at its alpha.
pub(crate) fn as_drawn(c: (u8, u8, u8), page: (u8, u8, u8)) -> (u8, u8, u8) {
    crate::anim::lerp_rgb(c, page, 1.0 - crate::plot::meter::track_alpha())
}

/// Walk `from` toward `to` in twentieths until what is DRAWN clears
/// [`floor`] against `page` — and, on a see-through window, against the
/// glass over any desktop ([`grounds`]). On a page `to` clears it by a wide
/// margin; over a light desktop's frost the footer's mid-ramp shade did not
/// (1.14), so the walk goes on from `to` toward the ink, and ends there.
///
/// Lifted rather than dropped, the way `crew_theme::tagcolor` lifts a tag
/// into readability: the recessed colour is the intent, and the floor is the
/// least of it that survives.
pub(crate) fn lift(from: (u8, u8, u8), to: (u8, u8, u8), page: (u8, u8, u8)) -> (u8, u8, u8) {
    let grounds = grounds(page);
    let reads = |c| {
        let on = |g: &(u8, u8, u8)| crew_theme::contrast_ratio(as_drawn(c, *g), *g);
        grounds.iter().all(|g| on(g) >= floor())
    };
    let ink = crew_theme::theme().ink;
    let step =
        |(a, b): ((u8, u8, u8), (u8, u8, u8)), k| crate::anim::lerp_rgb(a, b, k as f32 / 20.0);
    let legs = (0..=20).map(|k| step((from, to), k));
    legs.chain((1..=20).map(|k| step((to, ink), k)))
        .find(|&c| reads(c))
        .unwrap_or(ink)
}

/// What a track is laid down on: the page, and while the window is sheer
/// the frost over a black desktop and a white one. Measured on the page
/// alone, the chat footer's empty `ctx` track read 1.05:1 on light glass
/// over a light desktop — a gap where the meter should be (glass survey G6).
pub(crate) fn grounds(page: (u8, u8, u8)) -> Vec<(u8, u8, u8)> {
    let mut out = vec![page];
    if crew_theme::glassborder::sheer() {
        out.extend(crew_theme::glasslegend::pane_grounds(crew_theme::theme()));
    }
    out
}

#[cfg(test)]
#[path = "metertrack_tests.rs"]
mod tests;
