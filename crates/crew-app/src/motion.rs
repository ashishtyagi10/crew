//! How much crew moves: the user-facing motion strength.
//!
//! One knob, read the same way the glass level is — from the config, at the
//! point of use — so no animation has to thread a parameter down through every
//! call. Set in Settings → APPEARANCE → Motion.
//!
//! `Off` is a genuine off, not a fast setting: durations collapse to zero, so
//! every [`crate::ease::Timeline`] is born settled, draws its final state once,
//! and schedules no further frames.
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Process-wide motion strength, as an `AtomicU8` discriminant.
///
/// Read at the point of use rather than threaded through every scene call —
/// the same shape `palette::accent` and `crew_theme::theme` already use, and
/// the reason a new animation needs no plumbing to respect the setting.
static LEVEL: AtomicU8 = AtomicU8::new(2); // Full — matches `default_motion`

/// Whether the OS is asking for reduced motion, as last probed.
///
/// Cached rather than asked at the point of use because the probe is an
/// Objective-C round trip and [`MotionPref::resolve`] is read while laying out
/// frames; the answer only changes when the user visits System Settings, so
/// [`set_os_reduce`] republishes it exactly where the appearance sources are
/// republished (startup, config adoption, the appearance notification).
static OS_REDUCE: AtomicBool = AtomicBool::new(false);

/// Publish the OS "reduce motion" answer. Called from
/// `CrewConfig::publish_appearance_sources`, so every path that re-reads OS
/// preferences also refreshes this one.
pub(crate) fn set_os_reduce(reduce: bool) {
    OS_REDUCE.store(reduce, Ordering::Relaxed);
}

/// The last-published OS "reduce motion" answer.
pub(crate) fn os_reduce() -> bool {
    OS_REDUCE.load(Ordering::Relaxed)
}

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

/// What the user *chose* — as opposed to [`MotionLevel`], which is the
/// strength that actually renders.
///
/// The same split the theme makes between a `Selection` and a live palette,
/// and for the same reason: `auto` is not a strength, it is a deferral. It
/// says "ask the OS", and the OS answer can change under a running crew
/// without the config being touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MotionPref {
    /// Follow the OS accessibility switch: reduced → `Off`, otherwise `Full`.
    Auto,
    Fixed(MotionLevel),
}

impl MotionPref {
    /// Offer order for the settings picker and `/motion`, `auto` first because
    /// it is the default and the one most users should stay on. Built out of
    /// [`MotionLevel::ALL`] so the two lists cannot drift: a level added there
    /// and forgotten here would be parseable but never offered.
    pub(crate) const ALL: [MotionPref; 4] = [
        MotionPref::Auto,
        MotionPref::Fixed(MotionLevel::ALL[0]),
        MotionPref::Fixed(MotionLevel::ALL[1]),
        MotionPref::Fixed(MotionLevel::ALL[2]),
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            MotionPref::Auto => "auto",
            MotionPref::Fixed(l) => l.as_str(),
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("auto") || s.eq_ignore_ascii_case("system") {
            return Some(MotionPref::Auto);
        }
        MotionLevel::parse(s).map(MotionPref::Fixed)
    }

    /// The strength this preference renders at, given the OS answer.
    ///
    /// Only `Auto` consults `reduce` — a user who explicitly picked `full`
    /// has overruled the OS, and silently ignoring that would make the
    /// setting a lie.
    pub(crate) fn resolve(self, reduce: bool) -> MotionLevel {
        match self {
            MotionPref::Auto if reduce => MotionLevel::Off,
            MotionPref::Auto => MotionLevel::Full,
            MotionPref::Fixed(l) => l,
        }
    }

    /// Label for the settings picker: `auto` alone is opaque about what it
    /// currently *does*, so it names the strength it resolved to.
    pub(crate) fn label(self, reduce: bool) -> String {
        match self {
            MotionPref::Auto => format!("auto ({})", self.resolve(reduce).as_str()),
            MotionPref::Fixed(l) => l.as_str().to_string(),
        }
    }
}

#[cfg(test)]
#[path = "motion_tests.rs"]
mod tests;
