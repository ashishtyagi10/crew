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
    let iw = rect.width.saturating_sub(4);
    // An empty box says what empty means, in the muted ink, until it is
    // typed into: a blank `Accent` read as a value that had gone missing.
    let (text, fg) = match (text.is_empty(), hint) {
        (true, Some(h)) => (h.to_string(), dim()),
        _ => (text, ink()),
    };
    let line = Line::styled(tail(&text, iw as usize), Style::new().fg(fg));
    buf.set_line(rect.x + VALUE_X, rect.y + 1, &line, iw);
}

/// Where a box's value starts: under its legend's first letter (the legend
/// is ` Label ` from the corner), a column of air from the stroke as the
/// note on the right end has. At one column it touched the stroke, 4px off
/// it while the legend above sat a whole cell in (survey B #17).
pub(crate) const VALUE_X: u16 = 2;

/// `■ Label` single-row toggle — crew's drawn box, filled in the accent when
/// on, empty and quiet when off (it was `[x]`, typed text); `› ` marker +
/// accent bold when focused.
pub(crate) fn checkbox(buf: &mut Buffer, rect: Rect, label: &str, on: bool, focused: bool) {
    let (mark, mark_fg) = if on {
        ("\u{25a0}", focus_color())
    } else {
        ("\u{25a1}", dim())
    };
    let lead = if focused { "\u{203a} " } else { "  " };
    let mut style = Style::new().fg(if focused { focus_color() } else { ink() });
    if focused {
        style = style.add_modifier(Modifier::BOLD);
    }
    let line = Line::from(vec![
        Span::styled(lead, style),
        Span::styled(mark, style.fg(mark_fg)),
        // A label wider than its card says so with `…` rather than
        // stopping mid-word (`Language server diagnosti`).
        Span::styled(
            format!(
                " {}",
                crate::chatwidth::clip_w(label, usize::from(rect.width.saturating_sub(4)))
            ),
            style,
        ),
    ]);
    buf.set_line(rect.x, rect.y, &line, rect.width);
}

/// The form's two buttons, as drawn: the label padded inside its own fill,
/// the same width `[ Save ⌘S ]` was, so the click targets did not move. The
/// pads are no-break spaces: a bare space with a fill is emitted as a `█`
/// glyph, which squares the capsule's ends off. Save names its chord the way
/// `/keys` does on this platform: the `⌘` it wore on Windows and Linux named
/// a key those keyboards do not have, and so does Cmd (the Windows key) —
/// there it is Alt+S, the shorter of the two chords that save.
pub(crate) const SAVE: &str = if cfg!(target_os = "macos") {
    "\u{a0}\u{a0}Save\u{a0}\u{2318}S\u{a0}\u{a0}"
} else {
    "\u{a0}\u{a0}Save\u{a0}Alt+S\u{a0}\u{a0}"
};
pub(crate) const CANCEL: &str = "\u{a0}\u{a0}Cancel\u{a0}esc\u{a0}\u{a0}";

/// A button: `text` on a filled capsule (the renderer rounds a run of fill
/// bordered by page). The primary one sits on the accent; the other on a
/// quiet tint of the ink. Focus bolds the label (and deepens Cancel's); the
/// label is walked to the text floor on whatever fill it lands on.
pub(crate) fn button(text: &str, focused: bool, primary: bool) -> Span<'static> {
    let t = crew_theme::theme();
    let rgb = |c: Color| match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => t.ink,
    };
    let (fg, bg) = match (primary, focused) {
        (true, _) => crate::segment::inked(rgb(focus_color())),
        (false, f) => {
            let bg = crate::anim::lerp_rgb(t.page_bg, t.ink, if f { 0.3 } else { 0.14 });
            let fg = crew_theme::readable::enforced(t.ink, bg, crew_theme::contrast::text_floor());
            (fg, bg)
        }
    };
    let mut style = Style::new()
        .fg(Color::Rgb(fg.0, fg.1, fg.2))
        .bg(Color::Rgb(bg.0, bg.1, bg.2));
    // The primary button's label always carries weight: in regular weight
    // on its fill, beside Cancel, Save read as the disabled one (survey #7).
    if focused || primary {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(text.to_string(), style)
}

/// Multi-line boxed text area (one entry per line); shows the tail when the
/// content overflows, cursor on the final line while focused.
pub(crate) fn text_area(buf: &mut Buffer, rect: Rect, label: &str, value: &str, focused: bool) {
    frame(buf, rect, label, focused);
    let ih = rect.height.saturating_sub(2) as usize;
    let iw = rect.width.saturating_sub(4);
    let mut lines: Vec<String> = value.split('\n').map(str::to_string).collect();
    if focused {
        if let Some(last) = lines.last_mut() {
            last.push('\u{2588}');
        }
    }
    let skip = lines.len().saturating_sub(ih);
    for (i, l) in lines.iter().skip(skip).take(ih).enumerate() {
        let line = Line::styled(tail(l, iw as usize), Style::new().fg(ink()));
        buf.set_line(rect.x + VALUE_X, rect.y + 1 + i as u16, &line, iw);
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

#[cfg(test)]
#[path = "widgets_tests.rs"]
mod tests;
