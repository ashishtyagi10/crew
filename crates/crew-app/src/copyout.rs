//! Cmd+Shift+C (Alt+' off a Mac): copy one command's output, and only it.
//!
//! Pasting a failed build into an issue or a chat meant dragging a selection
//! over a scrollback and hoping the edges landed right. kitty's
//! `copy_last_command_output`, iTerm2's "Select Output of Last Command" and
//! Warp's block "Copy output" do it with the shell's own marks; crew already
//! has them (OSC 133, [`crate::cmdspan`]), so this is a chord over the same
//! cut `??` and fix-with-smith use ([`crate::failctx`]).
//!
//! The command is the last finished one, or — when the view has been jumped
//! up to an earlier one (Cmd+Opt+↑, [`crate::cmdjump`]) — the one on screen.
//! Without shell integration there are no marks to cut by, and guessing from
//! the process poll would copy the wrong text confidently: it says so instead.
use crate::app::CrewApp;
use crate::cmdspan::{Span, Spans};
use crate::pane::PaneContent;

/// Said when no command can be cut out, so the user learns what to set up
/// rather than that "it did nothing".
pub(crate) const NO_MARKS: &str =
    "no command output to copy \u{2014} needs shell integration (OSC 133)";

/// Longest command line quoted in the notice before it is cut with `…`.
const NAME_CHARS: usize = 40;

/// The index in `spans` of the command to copy. `top` is the buffer line on
/// the window's top row while scrolled back, `None` on the live screen.
/// Only commands the shell reported finished (OSC 133 `D;code`) count: a
/// polled span has no honest end. Scrolled back, the newest one that began at
/// or above the top row is the command being read.
pub(crate) fn pick(spans: &Spans, top: Option<usize>) -> Option<usize> {
    let done = |s: &Span| s.exit.is_some();
    let at = spans
        .0
        .iter()
        .rposition(|s| done(s) && top.is_none_or(|t| s.from <= t));
    at
}

/// The `[from, to)` rows of `rows` as text: a soft-wrapped row continues the
/// next without a newline, and trailing blank lines are dropped (the prompt's
/// gap, not the command's output).
pub(crate) fn join(rows: &[(String, bool)], from: usize, to: usize) -> String {
    let to = to.min(rows.len());
    let mut out = String::new();
    for (i, (line, wrapped)) in rows[from.min(to)..to].iter().enumerate() {
        out.push_str(line);
        if i + from + 1 < to && !wrapped {
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

/// `name` for the notice: redacted (a flag can carry a secret), one line,
/// and short.
fn quoted(span: &Span) -> String {
    let line = span.line.clone().unwrap_or_else(|| span.name.clone());
    let line = crate::failredact::redact(&line).replace('\n', " ");
    match line.chars().count() > NAME_CHARS {
        true => format!(
            "{}\u{2026}",
            line.chars().take(NAME_CHARS).collect::<String>()
        ),
        false => line,
    }
}

impl CrewApp {
    /// The focused terminal's chosen command as `(its name, its output)`.
    pub(crate) fn command_output(&mut self) -> Result<(String, String), String> {
        let pane = self.panes.get_mut(self.focused).ok_or("no pane focused")?;
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        let PaneContent::Terminal(t) = &mut pane.content else {
            return Err("copy: focus a terminal pane".into());
        };
        let (now, off) = (t.pty.lines_seen(), t.pty.display_offset());
        let top = (off > 0).then(|| now.saturating_sub(usize::from(rows)).saturating_sub(off));
        let Some(span) = pick(&t.spans, top).map(|i| t.spans.0[i].clone()) else {
            return Err(NO_MARKS.into());
        };
        let lines = crate::dump::capture_rows(&mut t.pty, cols, rows);
        // Anchored on the end: the capture is capped, so its numbering can
        // start later than the buffer's own.
        let drop = now.saturating_sub(lines.len());
        // As `failctx::failure_of`: a span's numbers sit one past the rows
        // they bound, because the cursor is on the blank line after a newline.
        let (from, to) = Spans::range(&span, now);
        let (from, to) = (from.saturating_sub(1), to.saturating_sub(1));
        let text = join(&lines, from.saturating_sub(drop), to.saturating_sub(drop));
        match text.is_empty() {
            true => Err(format!(
                "{} printed nothing \u{2014} nothing copied",
                quoted(&span)
            )),
            false => Ok((quoted(&span), text)),
        }
    }

    /// The chord. `put` is the clipboard (a parameter so tests never touch
    /// the real one); it says whether the text was taken.
    pub(crate) fn copy_command_output_to(&mut self, put: &mut dyn FnMut(String) -> bool) {
        match self.command_output() {
            Err(why) => self.set_status(why),
            Ok((name, text)) => {
                let lines = text.lines().count();
                match put(text) {
                    true => self.set_status(crate::wording::copied(
                        lines,
                        &format!(" of {name}'s output"),
                    )),
                    false => self.set_status("clipboard unavailable"),
                }
            }
        }
    }

    /// [`Self::copy_command_output_to`] on the system clipboard.
    pub(crate) fn copy_command_output(&mut self) {
        self.copy_command_output_to(&mut |text| {
            arboard::Clipboard::new().is_ok_and(|mut cb| cb.set_text(text).is_ok())
        });
    }
}

#[cfg(all(test, unix))]
#[path = "copyout_tests.rs"]
mod tests;
