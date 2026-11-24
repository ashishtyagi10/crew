//! The third moment the advisor is for: a turn about to end on an answer
//! when it changed files and no check passed since.
//!
//! WHY: a turn's last word was its own. Live, a turn renamed two of three
//! files, never built, and answered that the rename was done; nothing read
//! the answer before it stood. Anthropic's advisor tool, Claude Code's
//! `/advisor` and Amp's oracle are all consulted "before declaring done";
//! crew's advisor only read a failure or a stall. So when the reply asks for
//! no tool, and the turn wrote a file (`sys:edit`, `sys:write_file`) with no
//! check run passing after it (`crate::checkrun`), the advisor reads the
//! turn once, and what it says goes back to the agent with one more round to
//! run the check or fix what it names before it answers again.
//!
//! Never when nothing was written, when a check passed after the last write,
//! or when the advisor already read a round since that write (a failed build
//! after it, a stall): the agent has heard from it about this change. Once a
//! turn, from the same [`super::PER_TURN`] reads the others draw on.

use crate::clipmiddle::clip_middle;
use crate::tools::ToolCall;

/// Chars of the answer the brief shows.
const ANSWER_CAP: usize = 1_500;

/// What follows the advice when it holds an answer back.
pub(super) const NOTE: &str = "You changed files this turn and no check has passed since. If \
     the advice holds, run the check or make the fix it names; then give your final answer again.";

/// The turn's writes and checks, in the order they ran.
#[derive(Default)]
pub(super) struct Done {
    /// Paths written since the last check that passed.
    edited: Vec<String>,
    /// The latest check: its command, whether it passed, and whether it ran
    /// after the latest write.
    check: Option<(String, bool, bool)>,
    /// The advisor read a round after the latest write.
    pub(super) heard: bool,
    /// This trigger has had its read.
    pub(super) asked: bool,
}

impl Done {
    /// Note one call that came back `ok` or not.
    pub(super) fn call(&mut self, call: &ToolCall, ok: bool) {
        let args = serde_json::from_str::<serde_json::Value>(&call.args).unwrap_or_default();
        let field = |k: &str| args[k].as_str().map(str::to_string);
        match (call.server.as_str(), call.tool.as_str()) {
            ("sys", "edit" | "write_file") if ok => {
                let path = field("path").unwrap_or_else(|| clip_middle(&call.args, 80));
                if !self.edited.contains(&path) {
                    self.edited.push(path);
                }
                self.heard = false;
                if let Some(c) = self.check.as_mut() {
                    c.2 = false;
                }
            }
            ("sys", "run") => {
                if let Some(cmd) = field("cmd").filter(|c| crate::checkrun::is_check(c)) {
                    if ok {
                        self.edited.clear();
                    }
                    self.check = Some((cmd, ok, true));
                }
            }
            _ => {}
        }
    }

    /// Whether an answer now is one to read first.
    pub(super) fn due(&self) -> bool {
        !self.edited.is_empty() && !self.heard && !self.asked
    }
}

/// What the advisor is shown before the answer stands: the request, the
/// turn's calls, what it changed, its last check, and the answer.
pub(super) fn brief(request: &str, calls: &[String], done: &Done, answer: &str) -> String {
    let check = match &done.check {
        None => "none ran this turn".to_string(),
        Some((cmd, ok, after)) => format!(
            "`{cmd}` {} {} the last change",
            if *ok { "passed" } else { "failed" },
            if *after { "after" } else { "before" }
        ),
    };
    format!(
        "You advise a coding agent that is about to end its turn. It changed files and no \
         check (a build, test or lint run) has passed since its last change. You only \
         read: you run nothing, and the agent decides what to do with what you say.\n\n\
         THE REQUEST:\n{}\n\nITS CALLS THIS TURN, IN ORDER:\n{}\n\n\
         FILES IT CHANGED SINCE THE LAST PASSING CHECK:\n{}\n\nITS LAST CHECK: {check}\n\n\
         ITS ANSWER:\n{}\n\n\
         In at most three short lines, say the one check that would show the change works \
         (the command), and anything the answer claims that its calls do not show. Plain \
         lines: no preamble, no code fences.",
        clip_middle(request, super::text::REQUEST_CAP),
        calls.join("\n"),
        done.edited.join("\n"),
        clip_middle(answer, ANSWER_CAP),
    )
}

impl super::Advisor {
    /// The call to make and its brief when an answer now is one to read
    /// first and the turn has a read left; `None` lets it stand.
    pub(super) fn before_done(&self, answer: &str) -> Option<(super::Ask, String)> {
        let mut s = self.state();
        if s.asked >= super::PER_TURN || !s.done.due() {
            return None;
        }
        s.done.asked = true;
        let ask = s.ask.get_or_insert_with(|| (self.find)()).clone()?;
        s.asked += 1;
        Some((ask, brief(&self.request, &s.calls, &s.done, answer)))
    }
}

#[cfg(test)]
#[path = "done_tests.rs"]
mod tests;
