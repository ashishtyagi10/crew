//! How far crew's gradient is allowed to wander from the theme's own colour:
//! the user-facing *gradient shift* knob.
//!
//! The page's wash, the dot lattice and every card's stroke are drawn between
//! the active theme's two poles. `crew_theme::poleshift` can rotate that pair
//! around the hue wheel; this is the ladder that says by how much, read the
//! same way [`crate::motion`] and the glass level are — from the config, at
//! the point of use.
//!
//! Three rungs rather than a typed number of degrees, because the useful
//! range is narrow and the interesting choice is a taste, not a measurement:
//!
//! * **off** — the poles are the theme's constants, forever. This is the
//!   pre-v0.18.33 look, and it is what every headless shot test sees.
//! * **subtle** — ±16°, the width of one colour's neighbourhood. A violet
//!   theme visits indigo and magenta and is never anything else.
//! * **lively** — ±38°, far enough that the two ends of the breath read as
//!   different lights on the same room, still short of the next primary.
//!
//! The *sign* is what makes it a breath rather than a drift: the offset is a
//! sine of the clock (see `washphase`), so the colour leans one way, comes
//! back through the theme's exact colour, and leans the other. A monotonic
//! rotation would eventually put every theme through every hue, which is a
//! screensaver, not a palette.
use std::sync::atomic::{AtomicU8, Ordering};

/// Process-wide gradient level, as an `AtomicU8` discriminant — the same
/// shape `motion::LEVEL` uses, and for the same reason: no gradient surface
/// should have to thread a parameter down to respect a setting.
static LEVEL: AtomicU8 = AtomicU8::new(1); // Subtle — matches `default_gradient`

/// Adopt `level` app-wide. Called from `apply_config`, so Save, session
/// restore and an external config edit all land here.
pub(crate) fn set_level(level: GradientLevel) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// The live gradient level.
pub(crate) fn level() -> GradientLevel {
    match LEVEL.load(Ordering::Relaxed) {
        0 => GradientLevel::Off,
        2 => GradientLevel::Lively,
        _ => GradientLevel::Subtle,
    }
}

/// How far the gradient's colour may lean from the theme's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum GradientLevel {
    Off = 0,
    Subtle = 1,
    Lively = 2,
}

impl GradientLevel {
    /// The names, in cycle order — shared by the settings picker and
    /// [`Self::parse`], so a level can never be selectable but unparseable.
    pub(crate) const ALL: [GradientLevel; 3] = [
        GradientLevel::Off,
        GradientLevel::Subtle,
        GradientLevel::Lively,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            GradientLevel::Off => "off",
            GradientLevel::Subtle => "subtle",
            GradientLevel::Lively => "lively",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "fixed" => GradientLevel::Off,
            "subtle" | "low" | "on" => GradientLevel::Subtle,
            "lively" | "high" | "full" => GradientLevel::Lively,
            _ => return None,
        })
    }

    /// Half-width of the hue breath, in degrees: the furthest the poles lean
    /// either side of the theme's own colour. Under
    /// `crew_theme::poleshift::MAX_SHIFT_DEG` by construction — that clamp is
    /// the backstop for a hand-edited config, not this ladder's job.
    pub(crate) fn span_deg(self) -> f32 {
        match self {
            GradientLevel::Off => 0.0,
            GradientLevel::Subtle => 16.0,
            GradientLevel::Lively => 38.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_round_trips() {
        for l in GradientLevel::ALL {
            assert_eq!(GradientLevel::parse(l.as_str()), Some(l));
        }
        assert_eq!(
            GradientLevel::parse(" LIVELY "),
            Some(GradientLevel::Lively)
        );
        assert_eq!(GradientLevel::parse("fixed"), Some(GradientLevel::Off));
        assert_eq!(GradientLevel::parse("rainbow"), None);
    }

    /// Off is a genuine off: no lean at all, so the theme's poles are its own
    /// bytes and idle frames stay identical.
    #[test]
    fn off_is_a_genuine_off() {
        assert_eq!(GradientLevel::Off.span_deg(), 0.0);
    }

    /// The ladder climbs, and never past what the theme layer will store.
    #[test]
    fn the_ladder_climbs_and_stays_in_range() {
        let _g = crate::app::theme_test_guard();
        let (o, s, l) = (
            GradientLevel::Off.span_deg(),
            GradientLevel::Subtle.span_deg(),
            GradientLevel::Lively.span_deg(),
        );
        assert!(o < s && s < l, "{o} {s} {l}");
        assert!(l <= crew_theme::poleshift::MAX_SHIFT_DEG, "{l}");
    }

    #[test]
    fn the_global_round_trips_every_level() {
        let prev = level();
        for l in GradientLevel::ALL {
            set_level(l);
            assert_eq!(level(), l);
        }
        set_level(prev);
    }
}
