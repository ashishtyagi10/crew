//! Form controls for the settings pane: bento cards, boxed inputs with the
//! label as a fieldset legend, checkboxes, and a multi-line text area — plus
//! the pure two-column layout geometry shared by the renderer and tests.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Widget};

use super::Field;
use crate::palette::accent_color;

/// Pane width below which the two card columns stack vertically.
pub(crate) const STACK_BELOW: u16 = 64;
/// Content rows inside the notify-patterns text area.
pub(crate) const TEXTAREA_ROWS: u16 = 4;

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
/// Window + Notifications right), otherwise one stacked column.
pub(crate) fn layout(cols: u16) -> FormLayout {
    let mut rects = Vec::new();
    let mut cards = Vec::new();
    if cols >= STACK_BELOW {
        let col_w = (cols - 4) / 2; // 1-col margins + 2-col gutter
        let (lx, rx) = (1, 1 + col_w + 2);
        let ah = appearance(&mut rects, lx, 0, col_w);
        cards.push(Card {
            title: "APPEARANCE",
            rect: Rect::new(lx, 0, col_w, ah),
        });
        let wh = window(&mut rects, rx, 0, col_w);
        cards.push(Card {
            title: "WINDOW",
            rect: Rect::new(rx, 0, col_w, wh),
        });
        let ny = wh + 1;
        let nh = notifications(&mut rects, rx, ny, col_w);
        cards.push(Card {
            title: "NOTIFICATIONS",
            rect: Rect::new(rx, ny, col_w, nh),
        });
        FormLayout {
            cards,
            rects,
            height: ah.max(ny + nh),
        }
    } else {
        let w = cols.saturating_sub(2);
        let mut y = 0;
        type Build = fn(&mut Vec<(Field, Rect)>, u16, u16, u16) -> u16;
        for (title, build) in [
            ("APPEARANCE", appearance as Build),
            ("WINDOW", window),
            ("NOTIFICATIONS", notifications),
        ] {
            let h = build(&mut rects, 1, y, w);
            cards.push(Card {
                title,
                rect: Rect::new(1, y, w, h),
            });
            y += h + 1;
        }
        FormLayout {
            cards,
            rects,
            height: y - 1,
        }
    }
}

/// Appearance card fields; returns the card height (content + border).
fn appearance(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    rects.push((Field::FontFamily, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::FontSize, Rect::new(ix, cy, half, 3)));
    rects.push((Field::PaperGrain, Rect::new(ix + half + 2, cy, half, 3)));
    cy += 3;
    rects.push((Field::Theme, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::Accent, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::PaperTexture, Rect::new(ix, cy, iw, 1)));
    cy += 1;
    cy + 1 - y
}

/// Window card fields; returns the card height.
fn window(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    rects.push((Field::NavWidth, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    for f in [Field::ShowNav, Field::Maximized] {
        rects.push((f, Rect::new(ix, cy, iw, 1)));
        cy += 1;
    }
    cy + 1 - y
}

/// Notifications card fields; returns the card height.
fn notifications(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    for f in [
        Field::Notify,
        Field::NotifyAgentDone,
        Field::NotifyBell,
        Field::NotifyExit,
    ] {
        rects.push((f, Rect::new(ix, cy, iw, 1)));
        cy += 1;
    }
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::NotifyMinSecs, Rect::new(ix, cy, half, 3)));
    cy += 3;
    rects.push((
        Field::NotifyPatterns,
        Rect::new(ix, cy, iw, 2 + TEXTAREA_ROWS),
    ));
    cy += 2 + TEXTAREA_ROWS;
    cy + 1 - y
}

/// Content inset inside a card border: x + 2, width − 4.
fn inner(x: u16, w: u16) -> (u16, u16) {
    (x + 2, w.saturating_sub(4))
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
    let legend = if active { accent_color() } else { dim() };
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
    let mut style = Style::new().fg(if focused { accent_color() } else { ink() });
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
    let col = if focused { accent_color() } else { dim() };
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
