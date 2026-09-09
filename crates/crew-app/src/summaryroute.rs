//! The footer's third line: the live routing mode and who is working right
//! now, as oh-my-posh segments, then what is RUNNING or — when nothing is —
//! the hints.
//!
//! Split from [`crate::chatsummary`] for the line cap, along the line
//! between the lines that are readings (1 and 2) and the one that is a
//! sentence with badges in it. The mode is a badge on the accent, each
//! working agent a small badge on its roster colour — the same colour its
//! card's gutter and name wear — and the tail stays plain text.
//!
//! The width budget is the one the other lines use ([`budget_by`]), by
//! display column: a badge is its label plus caps and pads, and on a narrow
//! pane the names go first, then the mode, and the work ids last.
use crate::chatsummary::{running_seg, Fg, FooterCtx};
use crate::segment::{self, Caps};
use crate::summaryfit::budget_by;

/// One footer cell: the character, its ink, and the block it sits on —
/// `None` is the page. Lines 1 and 2 carry no blocks ([`lift`]).
pub(crate) type FCell = (char, Fg, Option<Fg>);

/// A plain line, onto the page.
pub(crate) fn lift(line: Vec<(char, Fg)>) -> Vec<FCell> {
    line.into_iter().map(|(c, fg)| (c, fg, None)).collect()
}

/// `text` as plain cells in `fg`.
fn plain(text: &str, fg: Fg) -> Vec<FCell> {
    text.chars().map(|c| (c, fg, None)).collect()
}

/// `text` as a badge on `bg`, capped both ends on the page, in the page's
/// ink walked to the text floor.
pub(crate) fn badge(text: &str, bg: Fg) -> Vec<FCell> {
    segment::badge(text, segment::page_ink(bg), bg, Caps::BOTH)
        .into_iter()
        .map(|c| (c.c, c.fg, c.bg))
        .collect()
}

/// Display columns of a run of cells.
fn width(cells: &[FCell]) -> usize {
    cells.iter().map(|c| crate::chatwidth::char_w(c.0)).sum()
}

/// Line 3 for `fc` at `cols`, budgeted to fit.
///
/// It used to be built as one string and clipped, which on a 40-column pane
/// cut "enter runs it · esc discards it" in half — teaching the user how to
/// accept a plan and not how to decline it. Half an instruction is worse
/// than none. Priorities: the plan/running segments (0) always outlast the
/// mode (1), which outlasts the names, the `/stop` how and the hints (2/3);
/// ties break toward the right.
pub(crate) fn route_line(fc: &FooterCtx, cols: usize) -> Vec<FCell> {
    let th = crew_theme::theme();
    let (green, muted) = (th.ansi[10], th.text_muted);
    let accent = crate::palette::accent();
    let mode = match crate::chatinput::relay_target(fc.input, fc.agents) {
        Some(name) => format!("\u{25b6}\u{25b6} @{name} relay"),
        None => "\u{25b6}\u{25b6} swarm mode".to_string(),
    };
    let mut segs: Vec<(Vec<FCell>, u8)> = vec![(badge(&mode, accent), 1)];
    // Who is working right now, each name a badge in its roster colour so
    // it matches the chip grid and message cards — lit while its tokens
    // flow, dim once they stop (`summarypulse`); past three names the
    // count is the information.
    match fc.active.as_slice() {
        [] => {}
        names if names.len() > 3 => {
            segs.push((plain(&format!("{} agents working", names.len()), green), 2));
        }
        names => {
            for n in names {
                let block = crate::summarypulse::agent_block(fc.pulse, n);
                segs.push((badge(&format!("@{n}"), block), 2));
            }
        }
    }
    if fc.plan_pending {
        // A pending plan outranks everything else here: it is the only thing
        // on this line addressed TO the user. The keys go compact rather than
        // missing when the pane is narrow.
        let keys = if cols >= 60 {
            "enter runs it \u{00b7} esc discards it"
        } else {
            "enter/esc"
        };
        segs.push((plain("plan ready", green), 0));
        segs.push((plain(keys, green), 0));
    } else if let Some((what, how)) = running_seg(fc.running_tasks, cols) {
        segs.push((plain(&what, green), 0));
        if let Some(how) = how {
            segs.push((plain(&how, green), 2));
        }
    } else if fc.active.is_empty() {
        // Only show hints when there are no active agents and no running work.
        segs.push((plain("/ for constructs", muted), 2));
        segs.push((plain("@ to relay to an agent", muted), 3));
    }
    let mut out = Vec::new();
    for (i, seg) in budget_by(segs, cols, |s| width(s)).into_iter().enumerate() {
        if i > 0 {
            out.extend(plain(" \u{00b7} ", muted));
        }
        out.extend(seg);
    }
    out
}

#[cfg(test)]
#[path = "summaryroute_tests.rs"]
mod tests;
