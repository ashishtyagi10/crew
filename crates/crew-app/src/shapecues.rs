//! Say it with a shape as well as a colour.
//!
//! WCAG 1.4.1 (*Use of Color*) is one line long and easy to fail without
//! noticing: colour must never be the **only** thing carrying a piece of
//! information. About one man in twelve cannot separate red from green, every
//! colour cue vanishes on a monochrome CRT theme, and none of them survive a
//! screenshot pasted into a ticket in greyscale.
//!
//! Crew mostly passes already, and by accident of taste rather than by rule:
//! attention markers are distinct glyphs (`!`, `⚑`, `✓`, `⊗`, `?`) that happen
//! to share the bell colour, the broadcast prompt is `»` and not just magenta,
//! and every toast names itself in its legend. Two places did not:
//!
//! * the **load gauges**, where nominal / warning / critical is the fill
//!   colour and nothing else (the percentage says the number, but not which
//!   band it is in — that is the tier's whole job), and
//! * the **minimized strip's dot**, where a pane that is *working* and a pane
//!   that merely *said something recently* are both `●`, told apart by a
//!   brightness pulse — colour and motion, the two channels this is about.
//!   (The sidebar rows were already fine: a busy row draws a spinner.)
//!
//! macOS has the matching switch (Accessibility → Display → "Differentiate
//! without color"), read the same way its two siblings are, and the cues can
//! be pinned on or off regardless.
//!
//! Off by default-ish (that is: `auto`, which is off unless the OS says
//! otherwise) because a glyph in every gauge row is noise for a reader who
//! can see the colour. The rule is *never colour alone* for anyone who needs
//! it, not *always both* for everyone.
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

/// Publish the resolved answer (the OS switch, or the user's override).
pub(crate) fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Whether crew should be adding shape cues right now.
pub(crate) fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Whether the OS is asking apps to differentiate without colour.
#[cfg(target_os = "macos")]
pub(crate) fn os_asks() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace()
        .accessibilityDisplayShouldDifferentiateWithoutColor()
}

/// Non-macOS: no portable probe, so `auto` is off and the cues are reached
/// with `/shapes on`.
#[cfg(not(target_os = "macos"))]
pub(crate) fn os_asks() -> bool {
    false
}

/// The load band a gauge is in — the thing its fill colour says and nothing
/// else does. The percentage beside it gives the number, not the verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tier {
    Nominal,
    Warn,
    Critical,
}

impl Tier {
    /// The band `frac` falls in. Shares its thresholds with
    /// [`crate::gauges::fill_color`] by being what that function asks — one
    /// definition, so the colour and the mark can never disagree about which
    /// band a reading is in.
    pub(crate) fn of(frac: f32) -> Self {
        if frac < 0.7 {
            Tier::Nominal
        } else if frac < 0.9 {
            Tier::Warn
        } else {
            Tier::Critical
        }
    }

    /// The mark that rides after a gauge's label, or `None` at nominal (and
    /// whenever the cues are off).
    ///
    /// `!` and `‼` rather than two unrelated symbols: they are the same mark
    /// escalating, which is what the tiers are, and the difference is legible
    /// at one cell in any monospace face.
    pub(crate) fn mark(self) -> Option<char> {
        if !on() {
            return None;
        }
        match self {
            Tier::Nominal => None,
            Tier::Warn => Some('!'),
            Tier::Critical => Some('\u{203c}'),
        }
    }
}

/// The nav / strip dot for a pane, given whether it is working.
///
/// With the cues on, a working pane draws `\u{25d0}` (a half-filled circle —
/// visibly *partial*, which is what "in progress" looks like) against the
/// solid `\u{25cf}` of a pane that simply spoke recently. Off, both are the
/// solid dot they have always been, told apart by the busy pulse.
pub(crate) fn dot(busy: bool) -> char {
    if busy && on() {
        '\u{25d0}'
    } else {
        '\u{25cf}'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One definition of the bands, shared with the fill colour — the mark
    /// and the colour must never disagree about which band a reading is in.
    #[test]
    fn the_tiers_match_the_colour_thresholds() {
        assert_eq!(Tier::of(0.0), Tier::Nominal);
        assert_eq!(Tier::of(0.69), Tier::Nominal);
        assert_eq!(Tier::of(0.7), Tier::Warn);
        assert_eq!(Tier::of(0.89), Tier::Warn);
        assert_eq!(Tier::of(0.9), Tier::Critical);
        assert_eq!(Tier::of(1.0), Tier::Critical);
    }

    /// The cues appear only when asked for, and then they must actually
    /// distinguish — three tiers that all mark the same are no better than
    /// three tiers that all look the same.
    #[test]
    fn the_marks_appear_only_when_asked_and_tell_the_tiers_apart() {
        let _g = crate::app::motion_test_guard();
        set(false);
        for t in [Tier::Nominal, Tier::Warn, Tier::Critical] {
            assert_eq!(t.mark(), None, "{t:?} marked with the cues off");
        }
        set(true);
        assert_eq!(Tier::Nominal.mark(), None, "nominal is the quiet case");
        let (w, c) = (Tier::Warn.mark(), Tier::Critical.mark());
        assert!(w.is_some() && c.is_some());
        assert_ne!(w, c, "warning and critical must not share a mark");
        set(false);
    }

    /// Busy and merely-recent were both a solid dot, separated by a pulse —
    /// brightness, which is the channel this whole module exists to stop
    /// relying on.
    #[test]
    fn a_working_pane_gets_its_own_glyph_only_when_asked() {
        let _g = crate::app::motion_test_guard();
        set(false);
        assert_eq!(dot(true), dot(false), "off, both are the solid dot");
        set(true);
        assert_ne!(dot(true), dot(false), "on, working must look different");
        assert_eq!(dot(false), '\u{25cf}', "the quiet case never changes");
        set(false);
    }
}
