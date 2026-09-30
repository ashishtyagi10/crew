//! F6's "Rename or move" box, floated over the panels the way Far Manager
//! floats its dialog and the way Alt+F1's drive list already floats here.
//! Three rows: what is being moved, the input (the other panel's folder until
//! it is edited) with its caret, and what Enter will do. Child module of
//! `render.rs`; the logic is `moveto`.
use super::super::{Prompt, PromptKind};
use super::promptbar::prompt_text;
use super::*;
use crate::chatwidth::{char_w, str_w};

/// The box's widest and narrowest, in columns; it takes three fifths of the
/// pane between them, so a long path has room without covering a tile.
const WIDEST: u16 = 72;
const NARROWEST: u16 = 36;

/// What Enter does, longest first: the first that fits the box is shown.
const HINTS: [&str; 3] = [
    "Enter moves it there \u{b7} a new name renames \u{b7} Esc cancels",
    "Enter moves \u{b7} a new name renames",
    "Enter \u{b7} Esc",
];

pub(super) fn move_box(buf: &mut Buffer, area: Rect, prompt: &Prompt, focused: bool) {
    let PromptKind::Move { name, .. } = &prompt.kind else {
        return;
    };
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let muted = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let w = (area.width * 3 / 5)
        .clamp(NARROWEST, WIDEST)
        .min(area.width);
    let h = 5u16.min(area.height);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let box_area = Rect::new(x, y, w, h);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .title(Span::styled(
            "Rename or move",
            Style::new().fg(accent_color()),
        ))
        .style(Style::new().bg(bg));
    let inner = block.inner(box_area);
    Widget::render(ratatui::widgets::Clear, box_area, buf);
    block.render(box_area, buf);
    let room = inner.width as usize;
    let mut input = prompt_text("", &prompt.input, room);
    if !focused {
        input.pop(); // the caret `prompt_text` ends every row with
    }
    let hint = HINTS
        .iter()
        .find(|h| str_w(h) <= room)
        .copied()
        .unwrap_or("");
    let rows = [
        (clip(&format!("\u{2018}{name}\u{2019} to:"), room), ink),
        (input, ink),
        (hint.to_string(), muted),
    ];
    for (i, (text, fg)) in rows.into_iter().enumerate() {
        let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        if row.y >= inner.y + inner.height {
            break;
        }
        Paragraph::new(Line::from(Span::styled(text, Style::new().fg(fg).bg(bg))))
            .style(Style::new().bg(bg))
            .render(row, buf);
    }
}

/// `s` within `room` columns, its end replaced by `…` when it does not fit.
fn clip(s: &str, room: usize) -> String {
    if str_w(s) <= room {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        if w + char_w(c) + 1 > room {
            break;
        }
        w += char_w(c);
        out.push(c);
    }
    out.push('\u{2026}');
    out
}
