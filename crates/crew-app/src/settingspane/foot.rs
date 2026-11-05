//! Where the scrolled form is cut, a field is drawn whole or not at all.
//!
//! The viewport ends wherever the pane's height puts it, and that row used to
//! fall through the middle of a boxed field as often as not — a `Min secs`
//! legend and top stroke with nothing under them, or a `Line spacing` box
//! with its value row sliced off (UI survey #11). Half a box reads as a
//! drawing bug, not as "more below"; the fade and the `↓` already say that,
//! and they say it better over a blank stretch of card than over a broken
//! control.
//!
//! So a field the viewport's top or bottom edge would split is left out, and
//! its rows stay page inside its group's card. Per field, not per row: in two
//! columns the field beside it may well fit, and it stays. The click map
//! reads the same answer, so a click on that blank stretch can't land on a
//! field nobody sees.
use ratatui::layout::Rect;

/// Whether `r` (virtual rows) is split by a viewport of `viewport` rows
/// scrolled to `off`. A field wholly off screen isn't split — it simply isn't
/// there — and the focused field is never left out: one taller than the
/// viewport (the patterns box on a short tile) is still what you are typing
/// in, and `form::scroll_for` keeps its top in view.
pub(super) fn split(r: Rect, off: u16, viewport: u16, focused: bool) -> bool {
    let end = off + viewport;
    let shows = r.y < end && r.y + r.height > off;
    let whole = r.y >= off && r.y + r.height <= end;
    !focused && shows && !whole
}

#[cfg(test)]
#[path = "foot_tests.rs"]
mod tests;
