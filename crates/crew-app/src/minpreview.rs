//! What a minimized pane is doing, in one line.
//!
//! A thumbnail in the strip has said three things since it was written: which
//! pane it is (the title on its border), that something arrived while you were
//! away (the count), and that it is alive (the dot). All three are ways of
//! saying *something*, and the row between the marker and the count — the only
//! row the card has — was empty.
//!
//! The answer was already on screen: the pane's own last line. A `cargo watch`
//! that just went red, an agent CLI holding a question, a shell back at its
//! prompt — each of those reads at a glance from the line the pane is
//! currently showing, which is exactly what you would restore the pane to see.
use crew_render::CellView;
use crew_term::TermModel;

use crate::chatwidth::{clip_w, str_w};
use crate::pane::{Pane, PaneContent};

/// Column the preview starts on: the marker owns column 0, and a glyph and a
/// word with nothing between them read as one thing.
const LEFT: u16 = 2;

/// Below this many columns there is no room to say anything true — three
/// characters and an ellipsis is noise, and the title on the border is
/// already the better answer.
const MIN_ROOM: usize = 6;

/// The line a thumbnail shows for `p`, or `None` when the pane has nothing to
/// say (an empty screen, a chat with no messages, a pane kind whose whole
/// content is a picture nobody can fit on one row).
pub(crate) fn of(p: &Pane) -> Option<String> {
    match &p.content {
        PaneContent::Terminal(t) => tidy(&t.pty.last_line()?),
        PaneContent::Chat(c) => chat(c),
        _ => None,
    }
}

/// A chat pane is doing one of two things: waiting on an agent, or holding
/// what was last said. The first outranks the second — a pane that is working
/// is the one you might be about to restore.
fn chat(c: &crate::chat::ChatPane) -> Option<String> {
    if let Some(a) = c.active.first() {
        let what = a.tool.as_deref().unwrap_or("thinking");
        return tidy(&format!("{} \u{b7} {what}", a.name));
    }
    let last = c.messages.last()?;
    tidy(&last.text)
}

/// One row's worth of a pane's line: the first line with anything on it,
/// with runs of whitespace (a TUI's column padding) closed up so the words
/// survive the clip. `None` when nothing is left.
pub(crate) fn tidy(s: &str) -> Option<String> {
    let line = s.lines().find(|l| !l.trim().is_empty())?;
    let mut out = String::new();
    let mut gap = false;
    for c in line.chars() {
        // Whitespace FIRST: a tab is both a control character and a gap, and
        // dropping it as the former ran two columns of a table together.
        if c.is_whitespace() {
            gap = !out.is_empty();
            continue;
        }
        if c.is_control() {
            continue;
        }
        if gap {
            out.push(' ');
        }
        gap = false;
        out.push(c);
    }
    (!out.is_empty()).then_some(out)
}

/// The thumbnail's middle: `text` from [`LEFT`] up to a column of air before
/// the unread badge (`badge`, in columns; 0 when there is none).
///
/// Nothing is drawn rather than a clipped word when the room is gone — see
/// [`MIN_ROOM`]. The colour is the muted one because this is a glance at a
/// pane you are not reading, sitting under the title that names it.
pub(crate) fn cells(text: &str, cols: u16, badge: usize) -> Vec<CellView> {
    let right = match badge {
        0 => 0,
        w => w + 1,
    };
    let room = usize::from(cols)
        .saturating_sub(usize::from(LEFT))
        .saturating_sub(right);
    if room < MIN_ROOM || str_w(text) == 0 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut col = LEFT;
    let mut out = Vec::new();
    for c in clip_w(text, room).chars() {
        out.push(CellView {
            col,
            row: 0,
            c,
            fg: t.text_muted,
            bg: t.page_bg,
            ..Default::default()
        });
        col += crate::chatwidth::char_w(c) as u16;
    }
    out
}

#[cfg(test)]
#[path = "minpreview_tests.rs"]
mod tests;
