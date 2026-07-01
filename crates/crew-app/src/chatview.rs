//! Composes the crew pane's full cell view: status header (row 0), agent
//! roster (row 1 when known), role-styled message cards, and the input
//! composer on the bottom row. Tiny panes fall back to the plain layout.
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
    let mut cells = crate::chathdr::header_cells(
        cols,
        &pane.channel,
        pane.connected,
        pane.messages.len(),
        pane.is_busy(),
    );
    if top > 1 {
        cells.extend(crate::chatroster::roster_cells(cols, 1, &pane.agents));
    }
    if pane.messages.is_empty() {
        // The plain layout already renders the hint + composer for this case.
        let mut body = layout_cells(&[], &pane.input, cols, rows - top, 0, pane.connected);
        for c in &mut body {
            c.row += top;
        }
        cells.append(&mut body);
        return cells;
    }
    let msg_rows = rows - top - 1; // the bottom row belongs to the composer
    cells.extend(crate::chatmsgs::message_cells(
        &pane.messages,
        cols,
        msg_rows,
        top,
        pane.scroll,
    ));
    cells.extend(input_row_cells(&pane.input, cols, rows - 1));
    cells
}

/// The `> input` composer line at `row`.
fn input_row_cells(input: &str, cols: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    format!("> {input}")
        .chars()
        .take(cols as usize)
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row,
            c,
            fg: t.ink,
            bg: t.page_bg,
            bold: false,
            italic: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_row_renders_prompt_at_row() {
        let cells = input_row_cells("hi", 20, 7);
        let text: String = cells.iter().map(|c| c.c).collect();
        assert_eq!(text, "> hi");
        assert!(cells.iter().all(|c| c.row == 7));
    }

    #[test]
    fn input_row_clips_to_width() {
        let cells = input_row_cells("a long input line", 5, 0);
        assert_eq!(cells.len(), 5);
    }
}
