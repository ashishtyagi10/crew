//! Shift+Tab's approval modes, and the questions they raise, in a chat pane.
//!
//! The mode is the pane's own — every pane has its own broker. Shift+Tab
//! steps it (auto → accept edits → ask first → plan only → yolo → auto, the
//! order Claude Code's modes run in), the broker hears it as
//! `PluginCommand::Mode`, and the footer's mode line wears it. In any mode
//! but auto the broker may stop a tool call to ask (`PluginEvent::Approval`):
//! the question lands in the transcript and on the footer, and on an empty
//! composer Enter allows it and Esc refuses it — the keys the plan prompt
//! already answers to. `y`/`yes` and `n`/`no` typed and sent work too.
use crew_plugin::{ApprovalMode, PluginCommand};

use crate::chat::ChatPane;
use crate::chatkeys::{ChatAction, ChatInput};

/// One question the broker's tool call is blocked on.
pub(crate) struct Asking {
    pub id: String,
    pub question: String,
    /// When it arrived, on the animation clock: the broker gives up on it
    /// after `approval::DEFAULT_TIMEOUT_MS`, and so does the pane.
    pub since: u64,
}

/// The mode's name on the footer badge and in the transcript.
pub(crate) fn label(mode: ApprovalMode) -> &'static str {
    match mode {
        ApprovalMode::Auto => "auto-approve",
        ApprovalMode::Edits => "accept edits",
        ApprovalMode::Ask => "ask first",
        ApprovalMode::Plan => "plan only",
        ApprovalMode::Yolo => "yolo",
    }
}

/// The mode a word names, as `/approvals` takes it — each mode's footer label, its
/// one-word form, and the names other agents use for it.
pub(crate) fn parse(word: &str) -> Option<ApprovalMode> {
    Some(
        match word
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], " ")
            .as_str()
        {
            "auto" | "auto approve" | "default" => ApprovalMode::Auto,
            "edits" | "accept edits" | "accept" | "auto edit" => ApprovalMode::Edits,
            "ask" | "ask first" | "suggest" => ApprovalMode::Ask,
            "plan" | "plan only" | "read only" => ApprovalMode::Plan,
            "yolo" | "bypass" | "full auto" => ApprovalMode::Yolo,
            _ => return None,
        },
    )
}

/// The mode's one-word name — what `parse` reads back and the config keeps.
pub(crate) fn word(mode: ApprovalMode) -> &'static str {
    match mode {
        ApprovalMode::Auto => "auto",
        ApprovalMode::Edits => "edits",
        ApprovalMode::Ask => "ask",
        ApprovalMode::Plan => "plan",
        ApprovalMode::Yolo => "yolo",
    }
}

/// What the mode means, said once when it is chosen.
pub(crate) fn meaning(mode: ApprovalMode) -> &'static str {
    match mode {
        ApprovalMode::Auto => {
            "everything runs, but a force-push, `rm -rf` outside the project or `sudo` asks first"
        }
        ApprovalMode::Edits => "file edits run; a command that cannot be undone asks first",
        ApprovalMode::Ask => "anything that changes something asks first",
        ApprovalMode::Plan => "read-only: nothing is changed, the agent plans instead",
        ApprovalMode::Yolo => "nothing asks, ever: not even a force-push or `rm -rf /`",
    }
}

/// What the footer needs to say about approvals.
#[derive(Clone, Copy, Default)]
pub(crate) struct Footer<'a> {
    pub mode: ApprovalMode,
    /// The question waiting for an answer, if one is…
    pub asking: Option<&'a str>,
    /// …and how many more are queued behind it.
    pub more: usize,
}

impl ChatPane {
    /// Shift+Tab: the next mode, told to the broker and said in the pane.
    pub(crate) fn cycle_mode(&mut self) {
        self.set_mode(self.approval_mode.next());
    }

    /// Start in `mode` (the saved default): told to the broker, nothing said.
    pub(crate) fn start_in(&mut self, mode: ApprovalMode) {
        self.approval_mode = mode;
        let _ = self.plugin.send(&PluginCommand::Mode { approval: mode });
    }

    /// Switch to `mode` (Shift+Tab, `/approvals`): told to the broker, said in the pane.
    pub(crate) fn set_mode(&mut self, mode: ApprovalMode) {
        self.approval_mode = mode;
        let _ = self.plugin.send(&PluginCommand::Mode { approval: mode });
        // Spelled as `/approvals` spells it, and drawn as this platform
        // writes the chord (⇧Tab on a Mac).
        let note = format!(
            "{} \u{2014} {}. Shift+Tab changes it",
            label(mode),
            meaning(mode)
        );
        self.push_note(crate::chordglyph::prose(&note).into_owned());
    }

    /// The broker is waiting on a yes or no for `question`. Several agents of
    /// one swarm can ask at once: they queue, answered first to last.
    pub(crate) fn ask_user(&mut self, id: String, question: String) {
        self.push_note(format!(
            "\u{26a0} allow this? {question} \u{2014} enter allows \u{00b7} esc refuses"
        ));
        self.asking.push_back(Asking {
            id,
            question,
            since: crate::anim::now_ms(),
        });
    }

    /// The question to answer next — the oldest the broker has not already
    /// given up on.
    pub(crate) fn asking(&self) -> Option<&Asking> {
        let timeout = crew_plugin::approval::DEFAULT_TIMEOUT_MS;
        let now = crate::anim::now_ms();
        self.asking
            .iter()
            .find(|a| now.saturating_sub(a.since) < timeout)
    }

    /// Answer the question [`Self::asking`] names. `false` when none waits.
    pub(crate) fn answer_approval(&mut self, granted: bool) -> bool {
        let timeout = crew_plugin::approval::DEFAULT_TIMEOUT_MS;
        let now = crate::anim::now_ms();
        self.asking
            .retain(|a| now.saturating_sub(a.since) < timeout);
        let Some(a) = self.asking.pop_front() else {
            return false;
        };
        let _ = self
            .plugin
            .send(&PluginCommand::Approve { id: a.id, granted });
        self.push_note(match granted {
            true => format!("allowed \u{2014} {}", a.question),
            false => format!("refused \u{2014} {}", a.question),
        });
        true
    }

    /// The keys the mode and a waiting question take before anything else
    /// sees them: `Some` when consumed, carrying what `on_input` returns.
    pub(crate) fn approval_key(&mut self, k: &ChatInput) -> Option<Option<ChatAction>> {
        match k {
            ChatInput::CycleMode => self.cycle_mode(),
            ChatInput::Enter if self.asking().is_some() => {
                let granted = match self.input.trim().to_ascii_lowercase().as_str() {
                    "" | "y" | "yes" => true,
                    "n" | "no" => false,
                    // Anything else is a message: it goes out as one.
                    _ => return None,
                };
                self.input.clear();
                self.answer_approval(granted);
            }
            ChatInput::Close if self.asking().is_some() && self.input.is_empty() => {
                self.answer_approval(false);
            }
            _ => return None,
        }
        Some(None)
    }

    /// What the footer shows about approvals.
    pub(crate) fn approval_footer(&self) -> Footer<'_> {
        Footer {
            mode: self.approval_mode,
            asking: self.asking().map(|a| a.question.as_str()),
            more: self.asking.len().saturating_sub(1),
        }
    }
}

#[cfg(test)]
#[path = "chatapprove_tests.rs"]
mod tests;
