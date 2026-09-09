//! Liveness motion for the crew pane's chrome: the "thinking" shimmer, the
//! agent-name pulse while tokens flow, and the idle connection dot's breath.
//!
//! Pure functions of a clock, like `chatreveal` — no timer, no per-frame
//! state — so the ambient 6 fps, the busy 15 fps and a 60 fps draw all land
//! the same colour at the same instant. Every colour handed back is floored
//! against the page it will sit on: a blend of two readable colours is not
//! itself guaranteed readable (luminance is not linear in sRGB), and the
//! floor is what makes the shimmer safe to put on any preset.
use crate::motion::MotionLevel;

pub(crate) type Color = (u8, u8, u8);

/// One sweep of the shimmer across its word, at full motion.
pub(crate) const SHIMMER_MS: u64 = 1_400;
/// Cells the highlight window spans, centre to centre of its soft edges.
const WINDOW: f32 = 5.0;
/// How long an agent's name stays lit after a burst of tokens.
pub(crate) const PULSE_MS: u64 = 250;
/// One breath of the idle connection dot, at full motion — and at Subtle,
/// where a slower breath is the quieter one.
pub(crate) const BREATH_MS: u64 = 3_200;
pub(crate) const BREATH_SUBTLE_MS: u64 = 5_000;

/// The shimmer's period at `level`: Subtle sweeps 1.6× slower; Off is 0,
/// meaning no sweep at all.
pub(crate) fn period(period_ms: u64, level: MotionLevel) -> u64 {
    match level {
        MotionLevel::Off => 0,
        MotionLevel::Subtle => period_ms * 8 / 5,
        MotionLevel::Full => period_ms,
    }
}

/// Where the highlight's centre sits at `now_ms`, in cells from the text's
/// first character. It enters from the left with its whole window off the
/// text and leaves the same way on the right, so every sweep starts and ends
/// on a plain word — the wrap is invisible.
fn centre(now_ms: u64, n: usize, period_ms: u64) -> f32 {
    let phase = (now_ms % period_ms) as f32 / period_ms as f32;
    -WINDOW / 2.0 + phase * (n as f32 + WINDOW)
}

/// Highlight weight of cell `i` for a window centred at `c`: 1 at the
/// centre, a cosine down to 0 half a window away, 0 beyond.
fn weight(i: usize, c: f32) -> f32 {
    let half = WINDOW / 2.0;
    let d = (i as f32 - c).abs();
    if d >= half {
        0.0
    } else {
        0.5 * (1.0 + (std::f32::consts::PI * d / half).cos())
    }
}

/// `text` with a soft highlight window sweeping left to right across it
/// once every `period_ms` (scaled by `level` — see [`period`]): each cell is
/// `base_fg` pulled toward `hi_fg` by its distance from the window's centre,
/// then floored against `page` at the text floor. Off is all `base_fg`.
pub(crate) fn cells(
    text: &str,
    now_ms: u64,
    base_fg: Color,
    hi_fg: Color,
    page: Color,
    period_ms: u64,
    level: MotionLevel,
) -> Vec<(char, Color)> {
    let n = text.chars().count();
    let p = period(period_ms, level);
    if p == 0 || n == 0 {
        return text.chars().map(|c| (c, base_fg)).collect();
    }
    let c = centre(now_ms, n, p);
    let floor = crew_theme::contrast::text_floor();
    text.chars()
        .enumerate()
        .map(|(i, ch)| {
            let mix = crate::anim::lerp_rgb(base_fg, hi_fg, weight(i, c));
            (ch, crew_theme::readable::against(mix, page, floor))
        })
        .collect()
}

/// How lit an agent's name is `now_ms` after its last token burst at
/// `last_ms`: 1 on the burst, easing out to 0 by [`PULSE_MS`] (scaled by
/// `level`). `None` (never a burst) and Off are 0 — the roster colour.
pub(crate) fn pulse_mix(last_ms: Option<u64>, now_ms: u64, level: MotionLevel) -> f32 {
    let dur = level.scale_ms(PULSE_MS);
    let Some(last) = last_ms else {
        return 0.0;
    };
    if dur == 0 {
        return 0.0;
    }
    let age = now_ms.saturating_sub(last) as f32;
    1.0 - crate::ease::out_cubic(age / dur as f32)
}

/// The header's agent name: its `roster` colour, brightened toward `ink` by
/// [`pulse_mix`], floored against `page`.
pub(crate) fn pulse_color(
    roster: Color,
    ink: Color,
    page: Color,
    last_ms: Option<u64>,
    now_ms: u64,
    level: MotionLevel,
) -> Color {
    let mix = pulse_mix(last_ms, now_ms, level);
    if mix <= 0.0 {
        return roster;
    }
    let want = crate::anim::lerp_rgb(roster, ink, mix);
    crew_theme::readable::against(want, page, crew_theme::contrast::text_floor())
}

/// The idle dot's breath at `now_ms`: 1 is fully lit (the `activity`
/// colour), 0 fully dim, a raised cosine between them over [`BREATH_MS`]
/// ([`BREATH_SUBTLE_MS`] at Subtle). Off is a constant 1 — the lit dot the
/// header has always drawn, and no frame ever asked for.
pub(crate) fn breath(now_ms: u64, level: MotionLevel) -> f32 {
    let period = match level {
        MotionLevel::Off => return 1.0,
        MotionLevel::Subtle => BREATH_SUBTLE_MS,
        MotionLevel::Full => BREATH_MS,
    };
    let phase = (now_ms % period) as f32 / period as f32;
    0.5 * (1.0 + (std::f32::consts::TAU * phase).cos())
}

/// The connection dot's colour while the pane is idle: `dim` to `activity`
/// by [`breath`], floored against `page` at the mark floor — a dot only has
/// to be seen.
pub(crate) fn breath_color(
    now_ms: u64,
    level: MotionLevel,
    dim: Color,
    activity: Color,
    page: Color,
) -> Color {
    let want = crate::anim::lerp_rgb(dim, activity, breath(now_ms, level));
    crew_theme::readable::against(want, page, crew_theme::readable::MARK_FLOOR)
}

#[cfg(test)]
#[path = "shimmer_tests.rs"]
mod tests;
