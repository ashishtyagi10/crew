//! Two composer commands the pane answers itself: `/approvals` and `/clear`.
//!
//! `/approvals` is Shift+Tab in words — `/approvals plan`, `/approvals yolo` — and `/approvals`
//! alone says which mode is on and what the others are. `/clear` is a fresh
//! conversation, as in Claude Code: the pane empties its transcript and drops
//! anything queued, and the broker (which hears it too) stops what runs and
//! forgets the thread.
use crate::chat::ChatPane;
use crate::chatapprove::{label, meaning, parse, word};
use crate::chatkeys::{ChatAction, ChatInput};

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
                self.push_note(format!(
                    "approval mode: {} \u{2014} {}. /approvals auto, edits, ask, plan or yolo \
                     changes it (so does Shift+Tab); /approvals default <mode> also makes it \
                     where new agent panes start",
                    label(mode),
                    meaning(mode)
                ));
            }
            ("/approvals", arg) => {
                let (default, name) = match arg.strip_prefix("default") {
                    Some(rest) if rest.is_empty() || rest.starts_with(' ') => (true, rest.trim()),
                    _ => (false, arg),
                };
                let Some(mode) = parse(name) else {
                    self.push_note(format!(
                        "no mode called \u{201c}{name}\u{201d} \u{2014} auto, edits, ask, plan or yolo"
                    ));
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
            _ => return None,
        }
        self.answered(&text, None)
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
