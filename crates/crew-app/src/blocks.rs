//! `/blocks`: what you ran in this pane, and how each one went.
//!
//! A pane's scrollback is one long column in which everything that ever ran
//! is mixed together, and the question people actually ask of it — *what did
//! I run in here, and which of them failed* — has to be answered by reading
//! it. Crew already knows: [`crate::cmdspan`] records every command's output
//! span from the foreground-process transitions it polls, and since OSC 133
//! it records the exit status a shell reports.
//!
//! So this is a listing, not a search. Each row is numbered the way `/out`'s
//! argument is numbered, which is the point of pairing them: `/blocks` says
//! what you ran and `/out 2` opens the output of the third one back.
//!
//! It is rendered to a temp file and opened in the file viewer like `/out`
//! and `/diff`, rather than printed into the pane — writing a summary of a
//! pane's history *into* that history is how a listing becomes something
//! else to scroll past.
use crate::cmdspan::{Span, Spans};
use crate::toolsrow::{fit, wrap, ROW_W};

/// Columns the elapsed field takes, so the command names line up.
const TIME_W: usize = 7;

/// Columns before the command: number, elapsed, outcome, and a space each.
///
/// The row is built to fit a TILE, like `/tools`' — a viewer opened as one
/// tile of a grid is nearer 47 columns than 80, and a command line is the
/// one column that must not wrap: `cargo test -p crew-app --bin crew` broken
/// after `--bin` reads as two commands. So the command gets what is left of
/// [`ROW_W`], cut in the middle when it is longer, and a cut command is
/// repeated whole on the indented lines under its row.
const NAME_AT: usize = 3 + 1 + TIME_W + 1 + 9 + 1;

/// How a block ended, in one glyph plus its status.
///
/// A block with no reported status is `·`, not `✓`: crew only knows how a
/// command ended when the shell says so, and drawing "no answer" as success
/// would be inventing one. A running block says so.
fn outcome(s: &Span) -> String {
    match (s.to.is_none(), s.exit) {
        (true, _) => "\u{25b8} running".into(),
        (_, Some(0)) => "\u{2713}".into(),
        (_, Some(code)) => format!("\u{2717} {code}"),
        (_, None) => "\u{b7}".into(),
    }
}

/// `1m04` / `12s` / `0.4s` — as long as it needs and no longer.
fn elapsed(ms: u64) -> String {
    let secs = ms / 1000;
    match secs {
        0 => format!("{}ms", ms.min(999)),
        1..=59 => format!("{secs}s"),
        _ => format!("{}m{:02}", secs / 60, secs % 60),
    }
}

/// The listing for one pane's spans, newest first, as viewer text.
///
/// `now` is the monotonic clock, so a block still running reports how long it
/// has been at it rather than nothing at all.
pub(crate) fn listing(spans: &Spans, title: &str, now: u64) -> String {
    let mut out = format!("# blocks \u{b7} {title}\n\n");
    let mut any = false;
    for (i, s) in spans.recent().enumerate() {
        any = true;
        let took = elapsed(Spans::elapsed_ms(s, now));
        let name = s.name.trim();
        let shown = fit(name, ROW_W - NAME_AT);
        out.push_str(&format!(
            "{i:>3} {took:>TIME_W$} {:<9} {shown}\n",
            outcome(s)
        ));
        if shown != name {
            for line in wrap(name, ROW_W - NAME_AT) {
                out.push_str(&format!("{:NAME_AT$}{line}\n", ""));
            }
        }
    }
    if !any {
        out.push_str("nothing has run in this pane yet\n");
        return out;
    }
    out.push_str("\nthe number is `/out <n>`: the output of that command, on its own.\n");
    out
}

impl crate::app::CrewApp {
    /// `/blocks` — open the focused pane's command history in the viewer.
    pub(crate) fn open_blocks(&mut self) {
        let focused = self.focused;
        let Some(pane) = self.panes.get(focused) else {
            self.set_status("blocks: no pane focused");
            return;
        };
        let title = pane.title_text();
        let crate::pane::PaneContent::Terminal(t) = &pane.content else {
            self.set_status("blocks: not a terminal pane");
            return;
        };
        let text = listing(&t.spans, &title, crate::anim::now_ms());
        // The same per-pane temp file `/out` uses, under its own name: two
        // panes' histories must not fight over one file, and re-running
        // `/blocks` in one pane overwrites rather than litters.
        let path = crate::lastout::temp_path(focused, "blocks");
        if let Err(e) = std::fs::write(&path, text) {
            self.set_status(format!("blocks: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view(&format!("blocks \u{b7} {title}"));
        self.mark_last_view_ephemeral(before);
    }
}

#[cfg(test)]
#[path = "blocksshot_tests.rs"]
mod shot;
#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
