//! How tightly crew packs the canvas — the user-facing information density.
//!
//! Every list-and-card tool that people live in eventually grows this knob
//! (mail clients, issue trackers, chat) because the right answer is not a
//! matter of taste so much as of screen: the same layout that reads as
//! generous on a 27" display wastes half a 13" laptop, and the same layout
//! that fits a laptop reads as cramped when it is the thing you stare at all
//! day. One setting, three steps.
//!
//! In a cell grid the honest units of density are **rows and gutters**, not
//! font metrics — the line height is the cell, and shrinking it is the font
//! size, which crew already has a separate knob for. So density moves the two
//! spaces that are genuinely empty:
//!
//! * the **gutter between pane cards** (and between the canvas and the
//!   sidebar, the input bar, the window edge — all one `gap`), and
//! * the **blank rows between chat cards**.
//!
//! Read at the point of use out of an atomic, like [`crate::motion`] and
//! `palette::accent`, so no layout has to thread a parameter down. That
//! matters more here than elsewhere: the gap feeds hit-testing as well as
//! rendering, and a density that reached one and not the other would put the
//! click target beside the thing it draws.
use std::sync::atomic::{AtomicU8, Ordering};

/// Process-wide density, as an `AtomicU8` discriminant.
static LEVEL: AtomicU8 = AtomicU8::new(1); // Cozy — matches `default_density`

/// Adopt `level` app-wide. Called from `apply_config`, so every path that
/// changes settings (Save, session restore, an external config edit) lands
/// here without having to know about density.
pub(crate) fn set_level(level: Density) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

/// The live density.
pub(crate) fn level() -> Density {
    match LEVEL.load(Ordering::Relaxed) {
        0 => Density::Compact,
        2 => Density::Roomy,
        _ => Density::Cozy,
    }
}

/// The gutter, in logical pixels, between every pane card and its neighbours.
/// The one place `chrome`'s layout math and the hit paths both read.
pub(crate) fn gap() -> f32 {
    level().gap_px()
}

/// How tightly crew packs the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Density {
    Compact = 0,
    Cozy = 1,
    Roomy = 2,
}

impl Density {
    /// The steps, in ladder order — shared by the settings picker and `parse`
    /// so a level can never be selectable but unparseable.
    pub(crate) const ALL: [Density; 3] = [Density::Compact, Density::Cozy, Density::Roomy];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Density::Compact => "compact",
            Density::Cozy => "cozy",
            Density::Roomy => "roomy",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "compact" | "tight" | "dense" => Density::Compact,
            "cozy" | "default" | "normal" => Density::Cozy,
            "roomy" | "comfortable" | "relaxed" => Density::Roomy,
            _ => return None,
        })
    }

    /// The gutter between pane cards, in logical px.
    ///
    /// Cozy is 8, which is what crew has always drawn — the step exists so
    /// that choosing the default changes nothing. Compact halves it rather
    /// than closing it: at zero the rounded strokes of two cards touch and
    /// the pair reads as one wide card with a line down it. Roomy stops at
    /// 14 because the gap is charged twice on an interior edge, and past
    /// that a four-pane grid starts losing a column of text to air.
    pub(crate) fn gap_px(self) -> f32 {
        match self {
            Density::Compact => 4.0,
            Density::Cozy => 8.0,
            Density::Roomy => 14.0,
        }
    }

    /// Blank rows between two unrelated chat cards.
    ///
    /// Zero is legible here in a way a zero gutter is not: each card opens
    /// with a header line carrying the sender's coloured gutter glyph, so the
    /// boundary is drawn in ink rather than in space. Chained replies of one
    /// task never take a spacer at any density — the tree connector is what
    /// says they belong together, and spacing them would contradict it.
    pub(crate) fn card_gap_rows(self) -> usize {
        match self {
            Density::Compact => 0,
            Density::Cozy => 1,
            Density::Roomy => 2,
        }
    }
}

#[cfg(test)]
#[path = "density_tests.rs"]
mod tests;
