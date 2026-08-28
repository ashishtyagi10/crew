//! One authority for how the left nav divides its rows.
//!
//! The offsets used to live as `+` chains in three places — the draw
//! (`statspane::cells`), the paint layer (`statspane::chart_paint`), and two
//! hit paths (`hit::pane_at_sidebar`, `navlogscroll`) — each re-deriving the
//! same sum. They agreed only by hand, and a section that changed height had
//! to be found in all four.
//!
//! It is also where the column's *slack* is spent. Everything above the LOG is
//! fixed furniture, and the pane list is as tall as there are panes; on a
//! full-height window that left a third of the nav empty below the last row,
//! with a LOG showing five lines onto sixty-four buffered ones. The LOG now
//! grows into whatever is left, so the space goes to the only section that has
//! more to say than it has room for.
use crate::clock;

/// Rows the SYSTEM section occupies: the rule, the three readings (arc gauges
/// on a wide nav, bars on a narrow one — both `sysdials::ROWS` tall so the
/// sections below never move when the nav is dragged), the CPU area chart,
/// and a gap.
pub const SYS_BLOCK: u16 = 1 + crate::sysdials::ROWS + CHART_ROWS + 1;
/// Rows the CPU chart occupies, and where it starts inside the SYSTEM block.
pub const CHART_ROWS: u16 = 2;
pub const CHART_OFF: u16 = 1 + crate::sysdials::ROWS;
/// Rows the LOAD section occupies (rule + 1 line + a one-row gap below it).
pub const LOAD_BLOCK: u16 = 3;
/// Rows a section with a rule + 2 content rows + one-row gap occupies (HOST, GIT).
pub const CARD_BLOCK: u16 = 4;
/// NET: rule + rates + the twin chart's two rows + gap.
pub const NET_BLOCK: u16 = 2 + crate::nettwin::ROWS + 1;

/// Fewest LOG lines worth a section. Below this the rule and its gap cost more
/// rows than the lines they introduce, so the section is dropped entirely.
pub const LOG_MIN: usize = 2;
/// Most LOG lines the nav will show, however tall the window is: past this the
/// section stops being a tail of recent activity and becomes a pane, which is
/// what `/log` is for.
pub const LOG_MAX: usize = 20;

/// Where each variable-height section of the nav starts, for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavLayout {
    /// Content row the LOG section's rule sits on.
    pub log_top: u16,
    /// Entry rows the LOG gets — 0 when it has nothing to show or no room.
    pub log_lines: usize,
    /// Content row the PANES header sits on.
    pub panes_top: u16,
}

impl NavLayout {
    /// Rows the LOG section occupies, rule and trailing gap included.
    pub fn log_block(&self) -> u16 {
        if self.log_lines == 0 {
            0
        } else {
            self.log_lines as u16 + 2
        }
    }
}

/// Rows above the LOG: everything fixed, plus GIT when the cwd is a repo.
pub fn fixed_rows(has_git: bool) -> u16 {
    clock::CLOCK_H
        + SYS_BLOCK
        + LOAD_BLOCK
        + CARD_BLOCK
        + NET_BLOCK
        + if has_git { CARD_BLOCK } else { 0 }
}

/// Divide a nav card of `rows` content rows between the LOG and the pane list.
///
/// The pane list is served first: it is the nav's navigation control, and a
/// row scrolled off it is a pane you cannot click. Whatever is left over goes
/// to the LOG, between [`LOG_MIN`] and [`LOG_MAX`] lines and never more than
/// there are entries to show.
pub fn layout(rows: u16, has_git: bool, log_len: usize, panes: usize) -> NavLayout {
    let log_top = fixed_rows(has_git);
    // Header + crew mix + one row per pane. An empty crew costs nothing.
    let panes_block = if panes == 0 {
        0
    } else {
        1 + crate::crewmix::ROWS + panes.min(usize::from(u16::MAX)) as u16
    };
    let slack = rows
        .saturating_sub(log_top)
        .saturating_sub(panes_block)
        .saturating_sub(2); // the LOG's own rule and trailing gap
    let log_lines = log_len.min(LOG_MAX).min(slack as usize);
    let log_lines = if log_lines < LOG_MIN { 0 } else { log_lines };
    let out = NavLayout {
        log_top,
        log_lines,
        panes_top: 0,
    };
    NavLayout {
        panes_top: log_top + out.log_block(),
        ..out
    }
}

#[cfg(test)]
#[path = "navlayout_tests.rs"]
mod tests;
