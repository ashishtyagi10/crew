//! How much arrived in a pane you were not looking at.
//!
//! A grid of panes means most of them are producing output while you are
//! reading one of the others, so each terminal pane remembers how many lines
//! its buffer held when you last read it. The difference rides the card's top
//! border as a count.
//!
//! **There used to be a rule as well** — a line drawn across the pane under
//! the last row you had read, answering *where* the new part starts rather
//! than only *how much* of it there is. It is gone, and it is worth saying
//! why, because the idea is a good one and the second attempt at it failed
//! for the same reason as the first.
//!
//! It was reported as "a weird line" twice. The first time, a bare full-width
//! rule read as damage — a rendering fault, a stray box-drawing character —
//! so it was given a `12 new` tag to name itself. The second report came with
//! a picture of the rule drawn **through an agent CLI's statusline**, and
//! that is the part no tag fixes: a program that repaints its own footer in
//! place grows the buffer by a line on every repaint, so the boundary lands
//! inside the live interface rather than between two lines of scrollback,
//! constantly, and the count it names is a repaint rather than news.
//!
//! Counting buffer lines is simply not "output arrived" for a program that
//! redraws itself — the same thing `blocked::TailWatch` learned when byte
//! quiescence could not tell waiting from thinking. The count survives
//! because it is honest at the granularity it is shown at (a badge saying
//! *something happened*), and the rule does not because it claims a position.

/// Where a pane's read mark sits this frame.
///
/// Watching is reading. The pane you are focused on, sitting at its live
/// bottom, has its output land in front of your eyes as it arrives — so its
/// mark follows the tail rather than counting output you are watching arrive.
/// Everything else keeps the mark it has: an
/// unfocused pane is the case this whole module exists for, and a focused
/// pane you have scrolled back in is one you are still catching up on.
///
/// This used to ask for `count(total, read_at) == 0` before advancing, which
/// is only ever true when the mark is *already* at the tail — a guard that
/// could not fire once a single line had arrived. The condition it was meant
/// to state is the one `scroll` and `termwrite` already state: at the live
/// bottom you have seen everything above you.
pub(crate) fn follow_tail(read_at: usize, focused: bool, at_bottom: bool, total: usize) -> usize {
    match focused && at_bottom {
        true => total,
        false => read_at,
    }
}

/// How many lines arrived since the pane was read. Saturating: a cleared
/// buffer (`/clear`) is shorter than it was, and that is not negative news.
pub(crate) fn count(total: usize, read_at: usize) -> usize {
    total.saturating_sub(read_at)
}

/// The count as it rides the card's top border. Capped, because a pane that
/// produced four thousand lines while you were away is saying "a lot" and
/// four digits of border says it no better than three.
pub(crate) fn badge(n: usize) -> Option<String> {
    match n {
        0 => None,
        1..=99 => Some(n.to_string()),
        _ => Some("99+".to_string()),
    }
}

#[cfg(test)]
#[path = "unread_tests.rs"]
mod tests;
