//! Composes the crew pane's full cell view: status header (row 0), agent
//! roster (row 1 when known), role-styled message cards, and the input
//! composer (affordance bar + prompt) on the bottom rows. Tiny panes fall
//! back to the plain layout.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chatlayout::layout_cells;

/// Render `pane` into a `cols` × `rows` grid.
pub(crate) fn cells(pane: &ChatPane, cols: u16, rows: u16) -> Vec<CellView> {
    let top = pane.top_rows(rows);
    if top == 0 {
        return layout_cells(
            &pane.messages,
            &pane.input,
            cols,
            rows,
            pane.scroll,
            pane.connected,
        );
    }
    let names = pane.active_names();
    let status = pane.active_status().map(|(label, secs)| {
        // One agent keeps its roster colour; a parallel pack goes accent.
        let color = match names.as_slice() {
            [one] => crate::chatroster::agent_color(one),
            _ => crate::palette::accent(),
        };
        (label, secs, color)
    });
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.messages.len(),
        pane.is_busy(),
        status.as_ref().map(|(l, s, c)| (l.as_str(), *s, *c)),
        pane.tokens,
    );
    if top > 1 {
        cells.extend(crate::chatroster::roster_cells(
            cols,
            1,
            &pane.agents,
            &names,
        ));
    }
    let bottom = crate::chatinput::composer_rows(rows);
    if pane.messages.is_empty() {
        cells.extend(crate::chatempty::empty_cells(
            cols,
            rows - bottom,
            top,
            pane.connected,
            &pane.agents,
        ));
    } else {
        let msg_rows = rows.saturating_sub(top + bottom);
        cells.extend(crate::chatmsgs::message_cells(
            &pane.messages,
            cols,
            msg_rows,
            top,
            pane.scroll,
        ));
        // Scroll affordances sit over the message area's last column/row.
        let total = crate::chatmsgs::card_line_count(&pane.messages, cols);
        cells.extend(crate::chatscroll::scrollbar_cells(
            total,
            msg_rows as usize,
            pane.scroll,
            cols.saturating_sub(1),
            top,
        ));
        if pane.scroll > 0 {
            let last = top + msg_rows.saturating_sub(1);
            cells.extend(crate::chatscroll::new_pill_cells(pane.unread, cols, last));
        }
    }
    cells.extend(crate::chatinput::composer_cells(
        &pane.input,
        &pane.agents,
        cols,
        rows,
    ));
    cells
}
