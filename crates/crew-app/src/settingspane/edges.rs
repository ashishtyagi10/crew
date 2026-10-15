//! The form's edges where it runs on past its viewport: the rows there fade
//! toward the page, and an arrow says which way the rest is. Split from
//! `render` for the 200-line cap.
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Style};
use ratatui::text::Line;

use super::form;

/// How far the cut rows lean toward the page: the edge row, then the row
/// inside it.
const FADE: [f32; 2] = [0.62, 0.3];

/// Fade the form where it is cut: while there is more below the viewport
/// (or above it), the rows at that edge lean toward the page, words and
/// strokes alike, so the cut reads as a list that scrolls. Cut square, the
/// group frames and the patterns field ended in four open strokes with only
/// a `↓` in the margin to say why (glass survey E12).
pub(super) fn fade(buf: &mut Buffer, cols: u16, viewport: u16, off: u16, total: u16) {
    let page = crew_theme::theme().page_bg;
    let mut lean = |y: u16, k: f32| {
        for x in 0..cols {
            if let Some(c) = buf.cell_mut((x, y)) {
                if let Color::Rgb(r, g, b) = c.fg {
                    let (r, g, b) = crate::anim::lerp_rgb((r, g, b), page, k);
                    c.fg = Color::Rgb(r, g, b);
                }
            }
        }
    };
    for (i, &k) in (0u16..).zip(FADE.iter()) {
        if i >= viewport {
            break;
        }
        if off + viewport < total {
            lean(viewport - 1 - i, k);
        }
        if off > 0 {
            lean(i, k);
        }
    }
}

/// `↑` / `↓` at the right edge when the form overflows the viewport, in the
/// margin outside the cards: one column in, they stood ON a card's border.
pub(super) fn hints(buf: &mut Buffer, cols: u16, viewport: u16, off: u16, total: u16) {
    let style = Style::new().fg(form::dim());
    let x = cols.saturating_sub(1);
    if off > 0 {
        buf.set_line(x, 0, &Line::styled("\u{2191}", style), 1);
    }
    if off + viewport < total {
        let y = viewport.saturating_sub(1);
        buf.set_line(x, y, &Line::styled("\u{2193}", style), 1);
    }
}

#[cfg(test)]
#[path = "edges_tests.rs"]
mod tests;
