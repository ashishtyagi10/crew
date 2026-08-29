//! Bridge from a ratatui `Buffer` to Crew's `CellView` grid. This lets panes use
//! ratatui's layout engine and widgets for in-pane structure while Crew keeps
//! rendering every cell on the GPU (and drawing its own rounded pane borders).
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

/// Convert a laid-out ratatui buffer into `CellView`s (origin-relative coords).
/// Fully-blank cells (a space with no background of its own, or the page's)
/// are skipped so we don't emit useless glyphs/quads.
///
/// A blank cell carrying a background that is NOT the page's own is a
/// different thing entirely: it is a **highlight** — a cursor bar, a selected
/// row, an inverted header — and dropping it shattered every one of them.
/// `/far`'s active-panel header drew as three disconnected green blocks with
/// its ` · ` separators knocked out, and its cursor bar covered the glyphs of
/// the selected row rather than the row. (The function-key bar already knew:
/// its pills are padded with half-block glyphs rather than spaces, a
/// workaround for exactly this, in the one place somebody noticed.)
pub fn to_cells(buf: &Buffer) -> Vec<CellView> {
    convert(buf, false)
}

/// Like [`to_cells`], but for overlays: blank cells that have a background colour
/// are emitted as a solid block glyph in that colour so the popup is opaque
/// (the renderer draws all backgrounds before all text, so a bare bg quad alone
/// lets text from panes behind bleed through).
pub fn to_cells_opaque(buf: &Buffer) -> Vec<CellView> {
    convert(buf, true)
}

fn convert(buf: &Buffer, opaque: bool) -> Vec<CellView> {
    let area = buf.area;
    let mut out = Vec::with_capacity((area.width as usize) * (area.height as usize));
    for y in 0..area.height {
        for x in 0..area.width {
            let Some(cell) = buf.cell((x, y)) else {
                continue;
            };
            let ch = cell.symbol().chars().next().unwrap_or(' ');
            let bg_opt = color_opt(cell.bg);
            let (c, fg) = if ch == ' ' {
                match (opaque, bg_opt) {
                    // Opaque overlay: paint blank cells as a solid block in their bg.
                    (true, Some(bg)) => ('█', bg),
                    // In-pane: only a blank coloured differently from the page
                    // is worth a quad — that one is a highlight, and it has to
                    // survive or the bar it belongs to comes apart.
                    (false, Some(bg)) if bg != crew_theme::theme().page_bg => ('█', bg),
                    _ => continue,
                }
            } else {
                (
                    ch,
                    color_opt(cell.fg).unwrap_or_else(|| crew_theme::theme().ink),
                )
            };
            out.push(CellView {
                col: x,
                row: y,
                c,
                fg,
                bg: bg_opt.unwrap_or_else(|| crew_theme::theme().page_bg),
                bold: cell.modifier.contains(Modifier::BOLD),
                italic: cell.modifier.contains(Modifier::ITALIC),
                ..Default::default()
            });
        }
    }
    out
}

/// Map a ratatui colour to RGB. `Reset` (and unsupported indexed colours) → `None`
/// so the caller can treat them as "unset".
fn color_opt(c: Color) -> Option<(u8, u8, u8)> {
    let rgb = match c {
        Color::Reset | Color::Indexed(_) => return None,
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (229, 229, 229),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
    };
    Some(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, BorderType, Widget};

    #[test]
    fn rounded_block_yields_corner_cells() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
        Block::bordered()
            .border_type(BorderType::Rounded)
            .render(buf.area, &mut buf);
        let cells = to_cells(&buf);
        assert!(cells.iter().any(|c| c.c == '╭'));
        assert!(cells.iter().any(|c| c.c == '╯'));
    }

    #[test]
    fn blank_buffer_yields_no_cells() {
        let buf = Buffer::empty(Rect::new(0, 0, 8, 2));
        assert!(to_cells(&buf).is_empty());
    }

    #[test]
    fn opaque_fills_blank_bg_cells_with_blocks() {
        use ratatui::style::{Color, Style};
        let _g = crate::app::theme_test_guard();
        // A popup's own sheet colour, which is the PAGE colour: the in-pane
        // path has nothing to say about it (there is nothing behind an
        // in-pane surface to bleed through), the overlay path fills it.
        let page = crew_theme::theme().page_bg;
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 2));
        buf.set_style(
            buf.area,
            Style::new().bg(Color::Rgb(page.0, page.1, page.2)),
        );
        assert!(to_cells(&buf).is_empty());
        let cells = to_cells_opaque(&buf);
        assert_eq!(cells.len(), 12);
        assert!(cells.iter().all(|c| c.c == '█' && c.fg == page));
    }

    /// A blank cell carrying a background that is not the page's own is a
    /// highlight — a cursor bar, a selected row, an inverted header — and
    /// dropping it took the middle out of every one of them.
    #[test]
    fn a_highlight_survives_its_own_spaces() {
        use ratatui::style::{Color, Style};
        let _g = crate::app::theme_test_guard();
        let page = crew_theme::theme().page_bg;
        let mut buf = Buffer::empty(Rect::new(0, 0, 7, 1));
        // "a b" on a bar: the space between the glyphs is part of the bar.
        let bar = Style::new()
            .fg(Color::Rgb(0, 0, 0))
            .bg(Color::Rgb(0, 200, 120));
        buf.set_string(0, 0, "a b", bar);
        let cells = to_cells(&buf);
        let bar_cells: Vec<char> = (0..3)
            .filter_map(|x| cells.iter().find(|c| c.col == x).map(|c| c.c))
            .collect();
        assert_eq!(bar_cells.len(), 3, "the bar came apart: {bar_cells:?}");
        assert_eq!(bar_cells[1], '█', "the space is filled, not dropped");

        // A blank at the PAGE colour is still nothing worth drawing.
        let mut plain = Buffer::empty(Rect::new(0, 0, 4, 1));
        plain.set_style(
            plain.area,
            Style::new().bg(Color::Rgb(page.0, page.1, page.2)),
        );
        assert!(
            to_cells(&plain).is_empty(),
            "a page-coloured blank is not a highlight"
        );
    }
}
