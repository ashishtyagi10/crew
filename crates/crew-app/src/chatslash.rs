//! The composer commands the pane answers itself: `/approvals`, `/clear`, `/init`.
//!
//! `/approvals` is Shift+Tab in words — `/approvals plan`, `/approvals yolo` — and `/approvals`
//! alone says which mode is on and what the others are. `/clear` is a fresh
//! conversation, as in Claude Code: the pane empties its transcript and drops
//! anything queued, and the broker (which hears it too) stops what runs and
//! forgets the thread.
use crate::chat::ChatPane;
use crate::chatapprove::{label, meaning, parse, word};
use crate::chatkeys::{ChatAction, ChatInput};

/// The modes, as a note lists them.
const MODES: &str = "auto, edits, ask, plan or yolo";

/// What `/init` asks for: the project's AGENTS.md (which crew, Codex and
/// others read in front of every task — `agentsmd`).
pub(crate) const INIT: &str = "Look over this project \u{2014} its layout, how to build, test and \
run it, its languages and conventions, and anything a newcomer would trip on \u{2014} and write \
what an agent needs to work on it into AGENTS.md at the project root, in under 80 lines. If \
AGENTS.md (or CLAUDE.md) already exists, read it first and improve it rather than replacing \
what a person wrote. Then say what you put in it.";

impl ChatPane {
    /// Enter on `/approvals …` or `/clear`: answered here. `Some` when consumed.
    pub(crate) fn slash_key(&mut self, k: &ChatInput) -> Option<Option<ChatAction>> {
        if !matches!(k, ChatInput::Enter) {
            return None;
        }
        let text = self.input.trim().to_string();
        let (head, arg) = text.split_once(' ').unwrap_or((text.as_str(), ""));
        match (head, arg.trim()) {
            ("/approvals", "") => {
                let mode = self.approval_mode;
                let note = format!(
                    "approval mode: {} \u{2014} {}. /approvals auto, edits, ask, plan or yolo \
                     changes it (so does Shift+Tab); /approvals default <mode> also sets the \
                     mode new agent panes start in",
                    label(mode),
                    meaning(mode)
                );
                self.push_note(crate::chordglyph::prose(&note).into_owned());
            }
            ("/approvals", arg) => {
                let (default, name) = match arg.strip_prefix("default") {
                    Some(rest) if rest.is_empty() || rest.starts_with(' ') => (true, rest.trim()),
                    _ => (false, arg),
                };
                let Some(mode) = parse(name) else {
                    self.push_note(match name {
                        // `/approvals default` alone: there is no name to quote back.
                        "" => format!("/approvals default <mode> \u{2014} {MODES}"),
                        _ => format!("no mode called \u{201c}{name}\u{201d} \u{2014} {MODES}"),
                    });
                    return self.answered(&text, None);
                };
                self.set_mode(mode);
                if default {
                    self.push_note(format!(
                        "new agent panes start in {} from now on",
                        label(mode)
                    ));
                    let saved = Some(ChatAction::DefaultMode(word(mode).to_string()));
                    return self.answered(&text, saved);
                }
            }
            ("/clear", "") => self.clear_conversation(),
            // Claude Code's /init: the instruction goes out as the message, so
            // the transcript says exactly what was asked.
            ("/init", "") => {
                self.history.record(&text);
                self.input.clear();
                self.submit_command(INIT.to_string());
                return Some(None);
            }
            // `/clear now`: sent on, it reached the broker as a construct
            // that was not one. Neither takes anything after it.
            ("/clear" | "/init", _) => self.push_note(format!(
                "{head} takes nothing after it \u{2014} send {head} alone"
            )),
            _ => return None,
        }
        self.answered(&text, None)
    }

    /// `/name rest`, where `name` is a skill (crew's own, or a Claude Code
    /// command) and no construct: the line becomes `@skill:name rest`, so the
    /// playbook rides along as the `@` picker's would, and is sent as usual.
    pub(crate) fn skill_command(&mut self, cwd: &std::path::Path) {
        let Some(line) = self.input.trim_start().strip_prefix('/') else {
            return;
        };
        let (name, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let slashed = format!("/{name}");
        if name.is_empty() || crate::chatcomplete::CONSTRUCTS.contains(&slashed.as_str()) {
            return;
        }
        if crew_plugin::skills_list(cwd).iter().any(|s| s.name == name) {
            self.input = format!("@skill:{name} {}", rest.trim())
                .trim_end()
                .to_string();
        }
    }

    /// A line answered here: recalled like any other, and gone from the composer.
    fn answered(&mut self, text: &str, action: Option<ChatAction>) -> Option<Option<ChatAction>> {
        self.history.record(text);
        self.input.clear();
        Some(action)
    }

    /// `/clear`: the transcript and the queue go, and the broker is told —
    /// past any queue, like `/stop`, since stopping what runs is half of it.
    fn clear_conversation(&mut self) {
        self.drop_queue();
        self.messages.clear();
        self.streaming.clear();
        self.swarm = None;
        self.folded = 0;
        self.scroll = 0;
        self.plan_pending = false;
        self.asking.clear();
        self.send_now("/clear".to_string());
    }
}

#[cfg(test)]
#[path = "chatslash_tests.rs"]
mod tests;
