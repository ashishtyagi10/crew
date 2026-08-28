//! Form controls for the settings pane: bento cards, boxed inputs with the
//! label as a fieldset legend, checkboxes, and a multi-line text area — plus
//! the pure two-column layout geometry shared by the renderer and tests.
//!
//! The left column is APPEARANCE alone because it is the tall one; the right
//! stacks WINDOW, NOTIFICATIONS and USAGE, which together roughly match it.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use super::cards::{appearance, notifications, usage, window};
use super::Field;
use crate::palette::focus_color;

/// Pane width below which the two card columns stack vertically.
pub(crate) const STACK_BELOW: u16 = 64;
/// Content rows inside the notify-patterns text area.
pub(crate) const TEXTAREA_ROWS: u16 = 4;

/// A card's field builder: places its rects and returns the card height.
type Build = fn(&mut Vec<(Field, Rect)>, u16, u16, u16) -> u16;

/// One bento card: a legend plus the frame the fields are drawn inside.
pub(crate) struct Card {
    pub(crate) title: &'static str,
    pub(crate) rect: Rect,
}

/// Computed form geometry, in virtual rows (y may exceed the pane height).
pub(crate) struct FormLayout {
    pub(crate) cards: Vec<Card>,
    pub(crate) rects: Vec<(Field, Rect)>,
    pub(crate) height: u16,
}

impl FormLayout {
    pub(crate) fn rect_of(&self, f: Field) -> Option<Rect> {
        self.rects.iter().find(|(g, _)| *g == f).map(|&(_, r)| r)
    }
}

/// Bento layout: two columns when the pane is wide enough (Appearance left;
/// Window + Notifications + Usage right), otherwise one stacked column.
pub(crate) fn layout(cols: u16) -> FormLayout {
    let mut rects = Vec::new();
    let mut cards = Vec::new();
    // Place one card and report the row after it, so adding a card is one
    // line rather than the five-line push-and-measure dance this repeated.
    let mut place = |rects: &mut Vec<(Field, Rect)>, title, build: Build, x, y, w| {
        let h = build(rects, x, y, w);
        cards.push(Card {
            title,
            rect: Rect::new(x, y, w, h),
        });
        y + h
    };
    if cols >= STACK_BELOW {
        let col_w = (cols - 4) / 2; // 1-col margins + 2-col gutter
        let (lx, rx) = (1, 1 + col_w + 2);
        let left = place(&mut rects, "APPEARANCE", appearance, lx, 0, col_w);
        let mut right = place(&mut rects, "WINDOW", window, rx, 0, col_w);
        for (title, build) in [("NOTIFICATIONS", notifications as Build), ("USAGE", usage)] {
            right = place(&mut rects, title, build, rx, right + 1, col_w);
        }
        FormLayout {
            cards,
            rects,
            height: left.max(right),
        }
    } else {
        let w = cols.saturating_sub(2);
        let mut y = 0;
        for (title, build) in [
            ("APPEARANCE", appearance as Build),
            ("WINDOW", window),
            ("NOTIFICATIONS", notifications),
            ("USAGE", usage),
        ] {
            y = place(&mut rects, title, build, 1, y, w) + 1;
        }
        FormLayout {
            cards,
            rects,
            height: y - 1,
        }
    }
}

/// Scroll offset keeping `rect` fully inside a `viewport`-row window over
/// `total` virtual rows (0 when everything fits).
pub(crate) fn scroll_for(rect: Rect, total: u16, viewport: u16) -> u16 {
    if viewport == 0 || total <= viewport {
        return 0;
    }
    (rect.y + rect.height)
        .saturating_sub(viewport)
        .min(rect.y)
        .min(total - viewport)
}

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
) {
    frame(buf, rect, label, focused);
    let mut text = value.to_string();
    if focused && cursor {
        text.push('\u{2588}');
    }
    let iw = rect.width.saturating_sub(2);
    let line = Line::styled(tail(&text, iw as usize), Style::new().fg(ink()));
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

/// Rounded input frame with the label as legend, accent while focused.
fn frame(buf: &mut Buffer, rect: Rect, label: &str, focused: bool) {
    let col = if focused { focus_color() } else { dim() };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(col))
        .title(Span::styled(format!(" {label} "), Style::new().fg(col)))
        .render(rect, buf);
}

/// The last `w` chars of `s`, so the cursor end stays visible while typing.
fn tail(s: &str, w: usize) -> String {
    let n = s.chars().count();
    s.chars().skip(n.saturating_sub(w)).collect()
}
