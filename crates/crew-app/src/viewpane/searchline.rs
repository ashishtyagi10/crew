//! The live search, drawn on the pane's last row: what you are typing and how
//! many lines hold it. Split from `render` for the line cap when the
//! diagnostics passes joined it.
//!
//! Without it the viewer's `/` was typed blind — the needle existed only in
//! the pane's state, so a mistyped search looked exactly like a search with no
//! matches, and neither said which it was.
use crew_render::CellView;

use super::ViewPane;

pub(super) fn search_line(out: &mut Vec<CellView>, p: &ViewPane, cols: u16, rows: u16) {
    let Some(s) = &p.search else { return };
    let t = crew_theme::theme();
    let row = rows - 1;
    // The matches themselves, washed where they sit — the terminal's `/find`
    // wash, over every row above this one. The line below counted `2 lines`
    // and nothing on screen said which two, or where on them.
    if !s.typing && !s.hits.is_empty() {
        crate::findhl::highlight(out, &s.needle, cols, row);
    }
    let count = match (s.typing, s.hits.len()) {
        (true, _) => String::new(),
        (false, 0) => "  no lines match".to_string(),
        (false, n) => format!("  {}", crate::wording::count(n, "line")),
    };
    let caret = if s.typing { "\u{2588}" } else { "" };
    let text = format!("/{}{caret}{count}", s.needle);
    // The row belongs to the search while it is open: clear whatever content
    // was drawn there rather than letting the two overprint.
    out.retain(|c| c.row != row);
    // Ink: this is the line you are typing into, and `hit_mark` is a gutter
    // tick's 3:1 colour — faint olive as text.
    let fg = match (s.typing, s.hits.is_empty()) {
        (false, true) => t.bell,
        _ => t.ink,
    };
    crate::chatwidth::place_row(0, cols, text.chars().map(|c| (c, fg)), |col, c, fg| {
        out.push(CellView {
            col,
            row,
            c,
            fg,
            bg: t.page_bg,
            ..Default::default()
        });
    });
}

#[cfg(test)]
#[path = "searchwash_tests.rs"]
mod wash_tests;
