//! How much crew moves: the user-facing motion strength.
//!
//! One knob, read the same way the glass level is — from the config, at the
//! point of use — so no animation has to thread a parameter down through every
//! call. Set in Settings → APPEARANCE → Motion.
//!
//! `Off` is a genuine off, not a fast setting: durations collapse to zero, so
//! every [`crate::ease::Timeline`] is born settled, draws its final state once,
//! and schedules no further frames.
use std::sync::atomic::{AtomicU8, Ordering};

/// Process-wide motion strength, as an `AtomicU8` discriminant.
///
/// Read at the point of use rather than threaded through every scene call —
/// the same shape `palette::accent` and `crew_theme::theme` already use, and
/// the reason a new animation needs no plumbing to respect the setting.
static LEVEL: AtomicU8 = AtomicU8::new(2); // Full — matches `default_motion`

/// Adopt `level` app-wide. Called from `apply_config`, so every path that
/// changes settings (Save, session restore, an external config edit) lands
/// here without having to know about motion.
pub(crate) fn set_level(level: MotionLevel) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// The live motion strength.
pub(crate) fn level() -> MotionLevel {
    match LEVEL.load(Ordering::Relaxed) {
        0 => MotionLevel::Off,
        1 => MotionLevel::Subtle,
        _ => MotionLevel::Full,
    }
}

/// Motion strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MotionLevel {
    Off = 0,
    Subtle = 1,
    Full = 2,
}

impl MotionLevel {
    /// The names, in cycle order — shared by the settings picker and `parse`
    /// so a level can never be selectable but unparseable.
    pub(crate) const ALL: [MotionLevel; 3] =
        [MotionLevel::Off, MotionLevel::Subtle, MotionLevel::Full];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            MotionLevel::Off => "off",
            MotionLevel::Subtle => "subtle",
            MotionLevel::Full => "full",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" => MotionLevel::Off,
            "subtle" | "low" => MotionLevel::Subtle,
            "full" | "on" => MotionLevel::Full,
            _ => return None,
        })
    }

    /// Scale an animation's nominal (full-motion) duration.
    ///
    /// Subtle runs at 60% — the same choreography, over quickly enough that it
    /// registers as responsiveness rather than as an effect.
    pub(crate) fn scale_ms(self, ms: u64) -> u64 {
        match self {
            MotionLevel::Off => 0,
            MotionLevel::Subtle => ms * 3 / 5,
            MotionLevel::Full => ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_round_trips() {
        for l in MotionLevel::ALL {
            assert_eq!(MotionLevel::parse(l.as_str()), Some(l));
        }
        assert_eq!(MotionLevel::parse(" FULL "), Some(MotionLevel::Full));
        assert_eq!(MotionLevel::parse("on"), Some(MotionLevel::Full));
        assert_eq!(MotionLevel::parse("swooshy"), None);
    }

    #[test]
    fn off_collapses_every_duration() {
        assert_eq!(MotionLevel::Off.scale_ms(1_000), 0);
        assert_eq!(MotionLevel::Off.scale_ms(1), 0);
    }

    /// The global round-trips every level — a mis-mapped discriminant would
    /// silently pin the whole app to one motion strength.
    #[test]
    fn the_global_round_trips_every_level() {
        let _g = crate::app::motion_test_guard();
        for l in MotionLevel::ALL {
            set_level(l);
            assert_eq!(level(), l);
        }
        set_level(MotionLevel::Full);
    }

    #[test]
    fn scaling_is_ordered() {
        let (o, s, f) = (
            MotionLevel::Off.scale_ms(500),
            MotionLevel::Subtle.scale_ms(500),
            MotionLevel::Full.scale_ms(500),
        );
        assert!(o < s && s < f, "{o} {s} {f}");
        assert_eq!(f, 500, "Full must not stretch the nominal duration");
    }
}
