//! Line spacing: how much air sits between one row of text and the next.
//!
//! Crew's cell height has always been `1.25 × font_size`. That is a good
//! default and a bad universal: dense code and long prose want different
//! amounts of air, small text at a distance wants more of it than large text
//! close up, and the reader who finds tight lines hard to track has, until
//! now, had only one lever — the font size — which fixes the tracking by
//! making everything bigger.
//!
//! ## Why it is not the same knob as density
//!
//! [`crate::density`] deliberately does *not* touch the line height, because
//! in a cell grid the line height IS the cell and shrinking it is the font
//! size, which has its own knob. That reasoning holds for the *gutters*
//! density moves; it does not hold for the reader who wants the same glyphs
//! with more room between them. This is that lever, kept separate on purpose:
//! density is about how much crew fits on the canvas, leading is about how
//! text reads.
//!
//! ## Height only
//!
//! Only the cell's HEIGHT takes the ratio. Widening the cell with it would
//! space the letters of every word apart — a different typographic decision
//! wearing the same name — and would break the monospace contract every
//! program in a pane is drawing against.
//!
//! No process-global, unlike [`crate::density`]: the cell box is asked for
//! in exactly two places — the renderer, which is handed the ratio when the
//! config is adopted, and [`crate::config::CrewConfig::line_height`], which
//! has the config in hand by definition. A global here would be a third
//! answer to a question that already has two agreeing ones.
/// How much air sits between rows of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leading {
    Tight,
    Normal,
    Relaxed,
    Loose,
}

impl Leading {
    /// The steps, in ladder order — shared by the settings picker and
    /// [`Self::parse`] so a level can never be selectable but unparseable.
    pub(crate) const ALL: [Leading; 4] = [
        Leading::Tight,
        Leading::Normal,
        Leading::Relaxed,
        Leading::Loose,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Leading::Tight => "tight",
            Leading::Normal => "normal",
            Leading::Relaxed => "relaxed",
            Leading::Loose => "loose",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "tight" | "dense" | "compact" => Leading::Tight,
            "normal" | "default" | "cozy" => Leading::Normal,
            "relaxed" | "comfortable" | "roomy" => Leading::Relaxed,
            "loose" | "airy" | "wide" => Leading::Loose,
            _ => return None,
        })
    }

    /// Cell height as a fraction of the font size.
    ///
    /// `Normal` is `1.25`, which is what crew has always drawn — the step
    /// exists so that choosing the default changes nothing. `Tight` stops at
    /// `1.10` rather than `1.0`: a monospace face's ascenders and descenders
    /// meet at the em box, so rows set solid have `g` touching the `T`
    /// beneath it. `Loose` stops at `1.65` because the cell is also the
    /// CURSOR and the selection band, and past that a highlighted row reads
    /// as a stripe with the text loose inside it rather than as a line.
    pub(crate) fn ratio(self) -> f32 {
        match self {
            Leading::Tight => 1.10,
            Leading::Normal => crew_render::CELL_H_RATIO,
            Leading::Relaxed => 1.45,
            Leading::Loose => 1.65,
        }
    }
}

#[cfg(test)]
#[path = "leading_tests.rs"]
mod tests;
