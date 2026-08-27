//! Which command's output you are looking at, named on the card's top border
//! while a pane is scrolled back.
//!
//! Scroll a terminal back far enough and the prompt that started what you are
//! reading is off the top of the window. The output is still there — pages of
//! it — with nothing on screen saying what produced it. Terminals that speak
//! OSC 133 answer this with a sticky prompt line pinned to the top of the
//! viewport; that costs a row of the program's grid, which is not crew's to
//! spend, and it needs the shell's cooperation, which crew has never asked
//! for.
//!
//! [`crate::cmdspan`] already knows where every command's output begins and
//! ends — it learns that from the foreground-process transitions `poll`
//! watches, no shell integration involved — so the answer is a lookup, and
//! the border is where crew says what it knows about a pane. The left border
//! already ticks `╶` where each command *began*; this is the same tick, on
//! the top border, with the name attached: the one whose block the top of the
//! window is currently inside.
//!
//! It draws only while scrolled back, beside the `⇡N` that appears under the
//! same condition — at the live bottom the prompt is on screen and answering
//! this question itself.
use crew_render::CellView;

use crate::panecard::put;

/// The tick the name hangs off — the same glyph the left border marks a
/// command's first row with, so the two read as one marking rather than two
/// features that happen to be about commands.
const TICK: char = '\u{2576}';

/// Narrowest useful badge: the tick, a space, and three characters of name.
/// Under this the name is initials, and an unreadable name on a border is
/// worse than a border.
const MIN_COLS: u16 = 5;

/// The badge for `name` in `avail` columns, or `None` when there is no room
/// to say anything legible. Long names lose their tail to an ellipsis rather
/// than their head: `cargo test --workspace…` still says what was run, while
/// `…--workspace` says only how it was run.
pub(crate) fn label(name: &str, avail: u16) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || avail < MIN_COLS {
        return None;
    }
    let room = usize::from(avail - 2); // the tick and its space
    let n = name.chars().count();
    Some(match n <= room {
        true => format!("{TICK} {name}"),
        // One column of the budget becomes the ellipsis, so the badge is
        // exactly `avail` wide either way.
        false => {
            let kept: String = name.chars().take(room - 1).collect();
            format!("{TICK} {kept}\u{2026}")
        }
    })
}

/// Stamp the badge on the top border, ending at column `rx` and never
/// reaching left of `min_col` (the legend's own last column plus one).
/// Returns the next free column to its left — `rx` unchanged when nothing
/// was drawn.
pub(crate) fn draw(v: &mut Vec<CellView>, rx: u16, min_col: u16, name: &str) -> u16 {
    let avail = rx.saturating_sub(min_col).saturating_add(1);
    let Some(s) = label(name, avail) else {
        return rx;
    };
    let w = s.chars().count() as u16;
    if rx < w {
        return rx;
    }
    let start = rx + 1 - w;
    // The command ticks' own colour: this names one of them.
    let fg = crew_theme::theme().legend_off;
    for (i, ch) in s.chars().enumerate() {
        put(v, start + i as u16, 0, ch, fg, false);
    }
    start.saturating_sub(2)
}

#[cfg(test)]
#[path = "cmdhead_tests.rs"]
mod tests;
