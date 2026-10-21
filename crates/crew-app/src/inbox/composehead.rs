//! The reply box's `to` and `cc` lines: a muted name, the addresses, and
//! on the line the caret is on, the caret, where a long line shows its end.
use super::compose::{Compose, Kind, On};
use super::draw::Canvas;

/// Where an address line's value starts, after `to` or `cc`.
const VALUE: u16 = 7;

/// The two lines, under the box's top edge at `top`, inside its `right` edge.
pub(super) fn draw(cv: &mut Canvas, c: &Compose, top: u16, right: u16) {
    let t = crew_theme::theme();
    for (i, (name, f, on)) in [("to", &c.to, On::To), ("cc", &c.cc, On::Cc)]
        .into_iter()
        .enumerate()
    {
        let row = top + 1 + i as u16;
        let focused = c.on == on;
        let label = if focused { t.ink } else { t.text_muted };
        cv.text(row, 3, VALUE, name, label, focused);
        let room = usize::from(right.saturating_sub(VALUE + 1));
        let (shown, at) = window(&f.text, f.at, room);
        match (shown.is_empty(), focused) {
            (true, true) => cv.text(row, VALUE, right - 1, hint(c, on), t.text_muted, false),
            _ => cv.text(row, VALUE, right - 1, &shown, t.ink, false),
        }
        if focused {
            caret(cv, c, (row, VALUE + at as u16), right);
        }
    }
}

/// The caret at `(row, col)`, unless the words are on their way.
pub(super) fn caret(cv: &mut Canvas, c: &Compose, (row, col): (u16, u16), right: u16) {
    if !c.sending {
        cv.text(row, col, right, "\u{258f}", crate::palette::accent(), false);
    }
}

/// What an empty place says.
pub(super) fn hint(c: &Compose, on: On) -> &'static str {
    match (on, c.kind) {
        (On::To, _) => "who it goes to",
        (On::Cc, _) => "add people, commas between",
        (On::Body, Kind::Forward) => "add a note, or send it as it is",
        (On::Body, _) => "write your reply \u{b7} tab: agent smith drafts it",
    }
}

/// The part of a line `room` columns wide that holds its caret, and the
/// caret's column in it: the start, or `…` and the end of a long line.
fn window(text: &str, at: usize, room: usize) -> (String, usize) {
    let w = crate::chatwidth::str_w;
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..at.min(chars.len())].iter().collect();
    if w(&before) + 1 < room {
        return (text.to_string(), w(&before));
    }
    let mut from = 0;
    while from < at && w(&chars[from..at].iter().collect::<String>()) + 2 >= room {
        from += 1;
    }
    let tail: String = chars[from..].iter().collect();
    let shown = format!("\u{2026}{tail}");
    (shown, 1 + w(&chars[from..at].iter().collect::<String>()))
}
