//! The reply box's `to` and `cc` lines: a muted name, the addresses, and on
//! the line the caret is on, the caret, where a long line shows its end.
use super::compose::{Compose, Kind, On};
use super::composedraw::X;
use super::composegeo::Geo;
use super::draw::Canvas;

/// Where an address line's value starts, after `to` or `cc`.
pub(crate) const VALUE: u16 = X + 4;

/// The lines on the rows `g` gives them, their words ending before column
/// `end`. The line being written shows the part around its caret; the
/// others show their start, cut at the edge.
pub(super) fn draw(cv: &mut Canvas, c: &Compose, g: &Geo, end: u16) {
    let t = crew_theme::theme();
    let lines = [
        (Some(g.to), "to", &c.to, On::To),
        (g.cc, "cc", &c.cc, On::Cc),
    ];
    for (row, name, f, on) in lines {
        let Some(row) = row else { continue };
        let focused = c.on == on;
        let label = if focused { t.ink } else { t.text_muted };
        cv.text(row, X, VALUE, name, label, focused);
        let room = usize::from(end.saturating_sub(VALUE));
        let (shown, at) = match focused {
            true => window(&f.text, f.at, room),
            false => (f.text.clone(), 0),
        };
        match (shown.is_empty(), focused) {
            // After the caret, so it does not stand on the first letter.
            (true, true) => cv.text(row, VALUE + 1, end, hint(c, on), t.text_muted, false),
            _ => cv.text(row, VALUE, end, &shown, t.ink, false),
        }
        if focused {
            caret(cv, c, (row, VALUE + at as u16), end);
        }
    }
}

/// The caret at `(row, col)`, unless the words are on their way.
pub(super) fn caret(cv: &mut Canvas, c: &Compose, (row, col): (u16, u16), end: u16) {
    if !c.sending {
        cv.text(
            row,
            col,
            end + 1,
            "\u{258f}",
            crate::palette::accent(),
            false,
        );
    }
}

/// What an empty place says.
pub(super) fn hint(c: &Compose, on: On) -> &'static str {
    match (on, c.kind) {
        (On::To, _) => "who it goes to",
        (On::Cc, _) => "add people, commas between",
        (On::Body | On::Files, Kind::Forward) => "add a note, or send it as it is",
        (On::Body | On::Files, _) => "write your reply \u{b7} tab: agent smith drafts it",
    }
}

/// The part of a line `room` columns wide that holds its caret, and the
/// caret's column in it: the start, or `…` and the end of a long line.
pub(super) fn window(text: &str, at: usize, room: usize) -> (String, usize) {
    let cw = crate::chatwidth::char_w;
    let chars: Vec<char> = text.chars().collect();
    let at = at.min(chars.len());
    let before: usize = chars[..at].iter().map(|&c| cw(c)).sum();
    if before + 1 < room {
        return (text.to_string(), before);
    }
    // Back from the caret, one pass: as much as leaves `…` and the caret room.
    let (mut from, mut wide) = (at, 0);
    while from > 0 && wide + cw(chars[from - 1]) + 2 < room {
        from -= 1;
        wide += cw(chars[from]);
    }
    let tail: String = chars[from..].iter().collect();
    (format!("\u{2026}{tail}"), 1 + wide)
}
