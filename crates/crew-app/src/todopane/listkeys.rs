//! What a key press does while the LIST has focus (a row is selected).
//!
//! Split out of [`super::keys`] for the line cap, along a seam the pane
//! already has: the composer's keys edit text, the list's keys act on the
//! item under the cursor and on what the list shows.
use super::keys::TodoInput;
use super::TodoPane;

/// Apply `input` to selected row `sel`. `cols`/`rows` are the pane's grid,
/// needed by the paging keys — a page is however many rows fit.
pub(crate) fn on_row(p: &mut TodoPane, sel: usize, input: TodoInput, cols: u16, rows: u16) {
    use TodoInput::*;
    match input {
        // In the done history Esc leaves the VIEW (the pane stays);
        // everywhere else it hands focus back to the composer.
        Close if p.done_view => p.set_done_view(false),
        Close | Tab => p.sel = None,
        Up => p.sel = (sel > 0).then(|| sel - 1),
        Down => p.sel = Some((sel + 1).min(p.visible_len().saturating_sub(1))),
        PageUp => p.sel = Some(p.page_target(sel, false, cols, rows)),
        PageDown => p.sel = Some(p.page_target(sel, true, cols, rows)),
        Home => p.sel = (p.visible_len() > 0).then_some(0),
        End => p.sel = (p.visible_len() > 0).then(|| p.visible_len() - 1),
        Left | Right | WordLeft | WordRight => {}
        Enter | Char(' ') => p.toggle_done_at(sel),
        Backspace | DeleteKey | Char('d') => p.delete_at(sel),
        // `e` would open an edit the history's composer can't submit, `h`
        // interleaves done rows a done-only view already shows, and `g`
        // bands by a person the history bands by day instead: all three
        // inert in there (and NOT composer-jump printables).
        Char('e') | Char('h') | Char('g') if p.done_view => {}
        Char('e') => p.edit_at(sel),
        // `H`: flip into (or out of) the done-history log.
        Char('H') => p.set_done_view(!p.done_view),
        Char(']') => p.cycle_filter(true),
        Char('[') => p.cycle_filter(false),
        Char('+') => p.bump_due_at(sel, true),
        Char('-') => p.bump_due_at(sel, false),
        // `h` (list only — in the composer it just types): show/hide done
        // items. Hiding clamps a selection left stranded past the shorter
        // list.
        Char('h') => p.set_show_done(!p.show_done),
        // `g`: band the list under each `#assignee` — the standup view.
        Char('g') => p.set_grouped(!p.grouped),
        // Any other printable jumps back to the composer and types.
        Char(c) => {
            p.sel = None;
            p.type_char(c);
        }
        Ignore => {}
    }
}

#[cfg(test)]
#[path = "listkeys_tests.rs"]
mod tests;
