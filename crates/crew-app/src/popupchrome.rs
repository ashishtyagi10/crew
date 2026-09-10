//! The chrome a composer pop-up is drawn in. A pop-up has the keys: every
//! arrow, Tab, Enter and Escape goes to it while it is open, and the pane
//! under it waits. It used to be drawn in the UNFOCUSED stroke with the
//! unfocused legend — the same grey frame as a pane nobody is looking at —
//! so nothing on the canvas said which box the next key would land in.
//! Here it wears what a focused pane wears: the focused stroke, and a bold
//! legend in the accent.
use crew_render::CellView;

/// A `cols × rows` fieldset card in the focused chrome, legend `title`.
pub(crate) fn card(cols: u16, rows: u16, title: &str) -> Vec<CellView> {
    let t = crew_theme::theme();
    let stroke = crate::panecardglow::focused_stroke(t);
    let accent = crate::palette::accent();
    let mut v = crate::boxdraw::titled_card(cols, rows, title, stroke, accent, t.page_bg);
    // The theme gradient at the focused stroke's brightness — static, so a
    // frame with an open pop-up repaints to the same bytes every time.
    crate::modernring::quiet(&mut v, cols, rows, stroke);
    for c in v.iter_mut().filter(|c| c.row == 0 && c.fg == accent) {
        c.bold = true;
    }
    v
}

/// Stamp `text` on the top border, right-aligned two columns in from the
/// corner, in the legend's accent: where a card says what it knows about
/// its own contents (`k/N` on a list longer than it shows). Skipped when
/// the card is too narrow to keep a column of frame either side.
pub(crate) fn mark(cells: &mut Vec<CellView>, text: &str, cols: u16) {
    let w = text.chars().count() as u16;
    if cols < w + 4 {
        return;
    }
    let (t, accent) = (crew_theme::theme(), crate::palette::accent());
    for (x, c) in (cols - 2 - w..).zip(text.chars()) {
        cells.push(CellView {
            col: x,
            row: 0,
            c,
            fg: accent,
            bg: t.page_bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    }
}

#[cfg(test)]
#[path = "popupchrome_tests.rs"]
mod tests;
