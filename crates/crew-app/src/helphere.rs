//! Which section of `/keys` is about the pane you are in, and where it sits.
//!
//! The panel lists every binding in crew: the global chords, then a section
//! per pane kind. Opening it from a Far panel put you at the top of a
//! seventy-line table with `in a /far file panel` four sections down — crew
//! knew which pane you were in and made you find it anyway. So the panel
//! opens ON that section, with the heading saying it is yours.
//!
//! The global chords are still one scroll up, which is the right way round:
//! they work here too, and you already know where "the top" is.
use crate::pane::PaneContent;

/// The section title for a pane kind, or `None` for a pane whose keys are
/// the global ones (a terminal passes everything through; the swarm, usage
/// and dash panes answer to the chords and nothing of their own).
pub(crate) fn section_for(content: &PaneContent) -> Option<&'static str> {
    Some(match content {
        PaneContent::Chat(_) => "in an agent pane",
        PaneContent::View(_) => "in the file viewer",
        PaneContent::Far(_) => "in a /far file panel",
        PaneContent::Todo(_) => "in the /todo pane",
        PaneContent::Settings(_) => "in /settings",
        PaneContent::Disk(_) => "in the /disk map",
        _ => return None,
    })
}

/// The row `title`'s heading sits on in an unfiltered panel `cols` wide, or
/// `0` when it is not there — the top is always a safe answer.
pub(crate) fn scroll_to(title: &str, cols: u16) -> usize {
    crate::helplayout::rows("", cols)
        .iter()
        .position(|r| matches!(r, crate::helplayout::Row::Head(h, _) if *h == title))
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "helphere_tests.rs"]
mod tests;
