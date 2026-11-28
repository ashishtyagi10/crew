//! Cmd+Shift+C (Alt+' off a Mac): copy one command's output, and only it.
//!
//! Pasting a failed build into an issue or a chat meant dragging a selection
//! over a scrollback and hoping the edges landed right. kitty's
//! `copy_last_command_output`, iTerm2's "Select Output of Last Command" and
//! Warp's block "Copy output" do it with the shell's own marks; crew reads
//! the same ones (OSC 133).
//!
//! The range comes from the buffer line the cursor was on when each mark
//! ARRIVED (`crew_term::PtyTerm::command_output`, which also says why the
//! rule is what it is), not from the once-a-second process poll that names
//! the spans on the border: the poll misses a command that is over before
//! it looks (`ls`), and notices the others up to a second late, which cut the
//! head off heavy output. So `ls` after `cargo test` copies `ls`.
//!
//! The command is the last finished one, or — scrolled back, by the wheel or
//! by Cmd+Opt+↑/↓ ([`crate::cmdjump`]) — the newest whose command line is at
//! or above the view's top row. Without shell integration there are no marks,
//! and guessing would copy the wrong text confidently: it says so instead.
//! Likewise when a full-screen program was up, the width changed since, or
//! the output was scrolled away: a notice, never wrong text.
//!
//! Not yet switched over: fix-with-smith ([`crate::failctx`]) still cuts by
//! the poll's spans.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crew_term::OutputErr;

/// Said when no command can be cut out, so the user learns what to set up
/// rather than that "it did nothing".
pub(crate) const NO_MARKS: &str =
    "no command output to copy \u{2014} needs shell integration (OSC 133)";

/// Longest command line quoted in the notice before it is cut with `…`.
const NAME_CHARS: usize = 40;

/// The command for a notice: redacted (a flag can carry a secret), one
/// line, and short.
fn quoted(command: &str) -> String {
    let line = crate::failredact::redact(command).replace('\n', " ");
    match (line.is_empty(), line.chars().count() > NAME_CHARS) {
        (true, _) => "the command".into(),
        (_, true) => format!(
            "{}\u{2026}",
            line.chars().take(NAME_CHARS).collect::<String>()
        ),
        _ => line,
    }
}

/// What to tell the user when `why` left nothing to copy.
fn refusal(why: OutputErr) -> String {
    let dash = "\u{2014}";
    match why {
        OutputErr::NoMarks => NO_MARKS.into(),
        OutputErr::AltScreen => format!(
            "copy: a full-screen program is up or ran there {dash} its screen is not output"
        ),
        OutputErr::Resized => format!(
            "copy: the pane was resized since {dash} its lines were rewrapped, nothing copied"
        ),
        OutputErr::Unsure => {
            format!("copy: a screen update held back where it ended {dash} nothing copied")
        }
        OutputErr::Nothing(c) => format!("{} printed nothing {dash} nothing copied", quoted(&c)),
        OutputErr::Gone(c) => format!(
            "{}'s output is gone from scrollback {dash} nothing copied",
            quoted(&c)
        ),
    }
}

impl CrewApp {
    /// The focused terminal's chosen command: its output, the notice
    /// suffix ` of <cmd>'s output…`, or why there is none.
    pub(crate) fn command_output(&mut self) -> Result<(String, String), String> {
        let pane = self.panes.get_mut(self.focused).ok_or("no pane focused")?;
        let PaneContent::Terminal(t) = &mut pane.content else {
            return Err("copy: focus a terminal pane".into());
        };
        let got = t.pty.command_output().map_err(refusal)?;
        let mut note = format!(" of {}'s output", quoted(&got.command));
        if got.dropped > 0 {
            let n = got.dropped;
            note += &format!(" (first {n} lines were dropped from scrollback)");
        }
        Ok((note, got.text))
    }

    /// The chord. `put` is the clipboard (a parameter so tests never touch
    /// the real one); it says whether the text was taken.
    pub(crate) fn copy_command_output_to(&mut self, put: &mut dyn FnMut(String) -> bool) {
        match self.command_output() {
            Err(why) => self.set_status(why),
            Ok((note, text)) => {
                let lines = text.lines().count();
                match put(text) {
                    true => self.set_status(crate::wording::copied(lines, &note)),
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
