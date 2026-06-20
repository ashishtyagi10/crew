use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::session::TerminalSession;

/// Render a minimized agent as a compact titled box (no PTY contents).
pub fn render_thumbnail(frame: &mut Frame, area: Rect, session: &TerminalSession) {
    let (glyph, color) = if !session.alive {
        ("✗", Color::Red)
    } else if session.has_attention {
        ("⚠", Color::Yellow)
    } else {
        ("●", Color::Indexed(240))
    };
    let label = format!(" {} {} ", glyph, session.title);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color).bg(Color::Black))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width > 0 && inner.height > 0 {
        frame.render_widget(Paragraph::new(label), inner);
    }
}
