//! What a failed command leaves behind, as one value: the command, how it
//! ended, where, and ITS OWN output.
//!
//! `??` used to hand the AI the last 120 lines of the screen, which after
//! `cargo test` fails is the failure, the prompt, and whatever was typed
//! since. Crew already knows where each command's output starts and stops
//! ([`crate::cmdspan`]) and, with an OSC 133 shell, how it ended — so the
//! failure can be cut out exactly. Pure: the caller pages the scrollback in.
use std::path::Path;

use crate::cmdspan::{Span, Spans};

/// Output kept for the model: the newest lines, because errors come last.
const KEEP_LINES: usize = 120;
const KEEP_BYTES: usize = 8 * 1024;

/// A command that exited non-zero.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub command: String,
    pub exit: i32,
    /// The pane's folder with `~` for home; `None` when the shell never said.
    pub cwd: Option<String>,
    /// Its output, ANSI stripped, the tail only.
    pub output: String,
}

/// The pane's most recent command, when the shell reported it failed. A
/// success, a silent shell (`exit` is `None`: not a success, but not known
/// either) and a command still running all answer `None`.
pub(crate) fn last_failed(spans: &Spans) -> Option<&Span> {
    spans
        .recent()
        .next()
        .filter(|s| s.exit.is_some_and(|c| c != 0))
}

/// The failure `span` records, cut from `text` (the pane's whole scrollback,
/// as `dump::capture_scrollback` returns it) after `now` lines. `None` when
/// the span did not fail.
pub(crate) fn failure_of(
    span: &Span,
    text: &str,
    now: usize,
    cwd: Option<&Path>,
) -> Option<Failure> {
    let exit = span.exit.filter(|c| *c != 0)?;
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    // The capture is capped, so its numbering can start later than the
    // buffer's own: anchor on the end, as `/out` does.
    let drop = now.saturating_sub(lines.len());
    // A span's numbers are `lines_seen` when the command started and when
    // the shell said it ended: one past the cursor's row, and the cursor sits
    // on the blank line after the last newline. The output is one line above.
    let (from, to) = Spans::range(span, now);
    let (from, to) = (from.saturating_sub(1), to.saturating_sub(1));
    let mut body =
        crate::lastout::slice(&lines, from.saturating_sub(drop), to.saturating_sub(drop));
    // A screen that has not scrolled yet counts no lines, so the span is
    // empty though the failure is on it: the screen's tail beats saying nothing.
    if body.trim().is_empty() {
        body = text.to_string();
    }
    let output = crate::askbar::context_tail(&strip_ansi(&body), KEEP_LINES, KEEP_BYTES);
    Some(Failure {
        command: span.line.clone().unwrap_or_else(|| span.name.clone()),
        exit,
        cwd: cwd.map(crate::disktile::short_path),
        output: output.trim_end().to_string(),
    })
}

/// `text` without terminal escapes (CSI, OSC and two-byte ones): a model
/// reads `error[E0433]`, not `ESC[1;31m` around it.
pub(crate) fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('[') => {
                for c in it.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = it.next() {
                    // BEL, or ESC \ (string terminator).
                    if c == '\u{7}' || (c == '\u{1b}' && it.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

impl Failure {
    /// `` `cargo test` failed (exit 101) in ~/code/crew``: the first line of
    /// both the composer prefill and the `??` context.
    fn headline(&self) -> String {
        let at = self
            .cwd
            .as_ref()
            .map(|c| format!(" in {c}"))
            .unwrap_or_default();
        format!("`{}` failed (exit {}){at}", self.command, self.exit)
    }

    /// The composer text for agent smith. A prefill, never a send: the user
    /// reads it, edits it, and presses Enter.
    pub(crate) fn prefill(&self) -> String {
        let ask = format!("{}. Find the cause and fix it.", self.headline());
        match self.output.is_empty() {
            true => ask,
            false => {
                // A fence longer than any backtick run inside the output.
                let run = self
                    .output
                    .split(|c| c != '`')
                    .map(str::len)
                    .max()
                    .unwrap_or(0);
                let fence = "`".repeat(run.max(2) + 1);
                format!("{ask}\n{fence}\n{}\n{fence}\n", self.output)
            }
        }
    }

    /// What `??` sends in place of the screen tail.
    pub(crate) fn explain_context(&self) -> String {
        format!("{}\n\n{}", self.headline(), self.output)
    }
}

#[cfg(test)]
#[path = "failctx_tests.rs"]
mod tests;
