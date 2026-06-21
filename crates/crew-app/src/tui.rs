//! Bridge from a ratatui `Buffer` to Crew's `CellView` grid. This lets panes use
//! ratatui's layout engine and widgets for in-pane structure while Crew keeps
//! rendering every cell on the GPU (and drawing its own rounded pane borders).
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

const DEFAULT_FG: (u8, u8, u8) = (220, 220, 220);
const DEFAULT_BG: (u8, u8, u8) = (8, 8, 16);

/// Convert a laid-out ratatui buffer into `CellView`s (origin-relative coords).
/// Fully-blank cells (a space with the default background) are skipped so we
/// don't emit useless glyphs/quads.
pub fn to_cells(buf: &Buffer) -> Vec<CellView> {
    let area = buf.area;
    let mut out = Vec::with_capacity((area.width as usize) * (area.height as usize));
    for y in 0..area.height {
        for x in 0..area.width {
            let Some(cell) = buf.cell((x, y)) else {
                continue;
            };
            let ch = cell.symbol().chars().next().unwrap_or(' ');
            let bg_opt = color_opt(cell.bg);
            if ch == ' ' && bg_opt.is_none() {
                continue;
            }
            out.push(CellView {
                col: x,
                row: y,
                c: ch,
                fg: color_opt(cell.fg).unwrap_or(DEFAULT_FG),
                bg: bg_opt.unwrap_or(DEFAULT_BG),
                bold: cell.modifier.contains(Modifier::BOLD),
                italic: cell.modifier.contains(Modifier::ITALIC),
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
}
