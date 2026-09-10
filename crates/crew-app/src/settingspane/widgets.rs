//! The form's widgets: a bento card, a boxed input, a checkbox, a text
//! area — each drawn into the ratatui buffer the form lays out in. Split
//! from [`super::form`] for the line cap, along the line between where a
//! field sits and how it is drawn.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use super::form::Card;
use crate::palette::focus_color;

pub(crate) fn dim() -> Color {
    let t = crew_theme::theme();
    Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2)
}

pub(crate) fn ink() -> Color {
    let t = crew_theme::theme();
    Color::Rgb(t.ink.0, t.ink.1, t.ink.2)
}

/// A bento card: rounded border, legend on the top edge (accent while the
/// focused field lives inside it).
pub(crate) fn card(buf: &mut Buffer, c: &Card, active: bool) {
    let legend = if active { focus_color() } else { dim() };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(dim()))
        .title(Span::styled(
            format!(" {} ", c.title),
            Style::new().fg(legend),
        ))
        .render(c.rect, buf);
}

/// A boxed input: rounded border with the label as legend; the focused box
/// gets an accent border and, for typed fields, a trailing block cursor.
pub(crate) fn input_box(
    buf: &mut Buffer,
    rect: Rect,
    label: &str,
    value: &str,
    focused: bool,
    cursor: bool,
    hint: Option<&str>,
) {
    frame(buf, rect, label, focused);
    let mut text = value.to_string();
    if focused && cursor {
        text.push('\u{2588}');
    }
    let iw = rect.width.saturating_sub(2);
    // An empty box says what empty means, in the muted ink, until it is
    // typed into: a blank `Accent` read as a value that had gone missing.
    let (text, fg) = match (text.is_empty(), hint) {
        (true, Some(h)) => (h.to_string(), dim()),
        _ => (text, ink()),
    };
    let line = Line::styled(tail(&text, iw as usize), Style::new().fg(fg));
    buf.set_line(rect.x + 1, rect.y + 1, &line, iw);
}

/// `[x] Label` single-row toggle; `› ` marker + accent bold when focused.
pub(crate) fn checkbox(buf: &mut Buffer, rect: Rect, label: &str, on: bool, focused: bool) {
    let mark = if on { "[x]" } else { "[ ]" };
    let lead = if focused { "\u{203a} " } else { "  " };
    let mut style = Style::new().fg(if focused { focus_color() } else { ink() });
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    let line = Line::styled(format!("{lead}{mark} {label}"), style);
    buf.set_line(rect.x, rect.y, &line, rect.width);
}

/// Multi-line boxed text area (one entry per line); shows the tail when the
/// content overflows, cursor on the final line while focused.
pub(crate) fn text_area(buf: &mut Buffer, rect: Rect, label: &str, value: &str, focused: bool) {
    frame(buf, rect, label, focused);
    let ih = rect.height.saturating_sub(2) as usize;
    let iw = rect.width.saturating_sub(2);
    let mut lines: Vec<String> = value.split('\n').map(str::to_string).collect();
    if focused {
        if let Some(last) = lines.last_mut() {
            last.push('\u{2588}');
        }
    }
    let skip = lines.len().saturating_sub(ih);
    for (i, l) in lines.iter().skip(skip).take(ih).enumerate() {
        let line = Line::styled(tail(l, iw as usize), Style::new().fg(ink()));
        buf.set_line(rect.x + 1, rect.y + 1 + i as u16, &line, iw);
    }
}

/// Rounded input frame with the label as legend — accent, and bold, while
/// focused: the box the next key lands in reads the way a focused pane's
/// legend does, and the colour alone was lost among a page of boxes.
fn frame(buf: &mut Buffer, rect: Rect, label: &str, focused: bool) {
    let col = if focused { focus_color() } else { dim() };
    let mut legend = Style::new().fg(col);
    if focused {
        legend = legend.add_modifier(Modifier::BOLD);
    }
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(col))
        .title(Span::styled(format!(" {label} "), legend))
        .render(rect, buf);
}

/// The last `w` chars of `s`, so the cursor end stays visible while typing.
fn tail(s: &str, w: usize) -> String {
    let n = s.chars().count();
    s.chars().skip(n.saturating_sub(w)).collect()
}
