//! Keyboard-shortcuts overlay (`/keys`): a centered rounded popup listing the
//! bindings, rendered with ratatui and dismissed by any key press.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Widget};

const ACCENT: Color = Color::Rgb(0, 255, 160);
const TEXT: Color = Color::Rgb(220, 220, 220);
const DIM: Color = Color::Rgb(150, 150, 160);
const PANEL: Color = Color::Rgb(18, 18, 30);

/// `(keys, description)` rows shown in the overlay.
const BINDINGS: &[(&str, &str)] = &[
    ("Ctrl+Tab / Ctrl+Shift+Tab", "Next / previous pane"),
    ("Cmd+1 … 9", "Jump to pane N"),
    ("Cmd+A", "Jump to next active pane"),
    ("Cmd+{ / Cmd+}", "Move pane left / right"),
    ("Cmd+I", "Focus the input bar"),
    ("Cmd+T", "New shell pane"),
    ("Cmd+, / Cmd+J", "Settings / chat pane"),
    ("Cmd+G", "Toggle sidebar"),
    ("Cmd+Z", "Zoom focused pane"),
    ("Cmd+S", "Broadcast to all panes"),
    ("Cmd+= / Cmd+- / Cmd+0", "Font size + / - / reset"),
    ("Cmd+C / Cmd+V", "Copy screen / paste"),
    ("Cmd+W / Cmd+M", "Close pane / maximize"),
    ("Shift+PageUp / PageDown", "Scroll focused pane"),
    ("Shift+Home / End", "Scroll to top / bottom"),
    ("/ (in input)", "Command palette"),
    ("Cmd+Q", "Quit"),
];

/// Preferred overlay size in cells.
pub fn size() -> (u16, u16) {
    (52, BINDINGS.len() as u16 + 4)
}

/// Render the help overlay into a `cols × rows` grid.
pub fn help_cells(cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 12 || rows < 4 {
        return Vec::new();
    }
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    let items: Vec<ListItem> = BINDINGS
        .iter()
        .map(|(keys, desc)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{keys:<26}"), Style::new().fg(ACCENT)),
                Span::styled(*desc, Style::new().fg(TEXT)),
            ]))
        })
        .collect();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .style(Style::new().bg(PANEL))
        .title(Span::styled(
            " keyboard shortcuts ",
            Style::new().fg(ACCENT),
        ));
    let inner = block.inner(buf.area);
    block.render(buf.area, &mut buf);
    List::new(items).render(inner, &mut buf);
    // dismissal hint on the bottom border
    let hint = " any key to close ";
    let hint_col = cols.saturating_sub(hint.len() as u16 + 2);
    for (i, ch) in hint.chars().enumerate() {
        let col = hint_col + i as u16;
        if let Some(cell) = buf.cell_mut((col, rows - 1)) {
            cell.set_char(ch).set_fg(DIM);
        }
    }
    crate::tui::to_cells_opaque(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bindings_with_border() {
        let (w, h) = size();
        let cells = help_cells(w, h);
        assert!(cells.iter().any(|c| c.c == '╭'));
        assert!(cells.iter().any(|c| c.c == 'C')); // e.g. "Ctrl+Tab"
    }

    #[test]
    fn tiny_renders_nothing() {
        assert!(help_cells(8, 3).is_empty());
    }
}
