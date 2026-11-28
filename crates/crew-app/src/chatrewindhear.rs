//! Rewind, pane side, part two: what the broker's events do — a task start
//! binds to the message that made it, an answer to our ask opens the
//! confirmation, and the final answer cuts the transcript (`chatrewind`).
use crew_plugin::{PluginCommand, PluginEvent};

use crate::chat::ChatPane;
use crate::chatrewind::{shown, Ask, Mark, LABEL};

impl ChatPane {
    /// Every broker event, for the two that matter here: a task start binds
    /// to the oldest message it was made from, and an answer to our ask.
    pub(crate) fn rewind_hears(&mut self, ev: &PluginEvent) {
        match ev {
            PluginEvent::Task {
                id,
                label,
                running: true,
            } => {
                let made_from = |m: &&mut Mark| {
                    m.task.is_none()
                        && label.starts_with(&m.text.trim().chars().take(LABEL).collect::<String>())
                };
                if let Some(m) = self.rewind.marks.iter_mut().find(made_from) {
                    m.task = Some(*id);
                }
            }
            PluginEvent::Rewind {
                task,
                files,
                outside,
                applied,
                error,
            } if self
                .rewind
                .want
                .as_ref()
                .is_some_and(|w| w.task == Some(*task)) =>
            {
                self.rewind_answer(*task, (*files, *outside, *applied), error);
            }
            _ => {}
        }
    }

    fn rewind_answer(
        &mut self,
        task: u64,
        (files, outside, applied): (u32, bool, bool),
        error: &str,
    ) {
        if !error.is_empty() {
            self.rewind.want = None;
            return self.push_note(format!("rewind: {error}"));
        }
        if applied {
            return self.rewind_done(files);
        }
        if files == 0 {
            return self.rewind_send(task, true); // the conversation alone: no file step
        }
        self.rewind.ask = Some(Ask {
            task,
            files,
            outside,
        });
    }

    pub(crate) fn rewind_send(&mut self, task: u64, confirm: bool) {
        let _ = self.plugin.send(&PluginCommand::Rewind { task, confirm });
    }

    /// Cut the transcript before the picked message and hand its text back.
    fn rewind_done(&mut self, files: u32) {
        let Some(m) = self.rewind.want.take() else {
            return;
        };
        let at = self
            .messages
            .iter()
            .rposition(|x| x.sender == "user" && x.ts == m.ts);
        if let Some(i) = at.filter(|&i| i > 0 || self.folded == 0) {
            self.messages.truncate(i);
        }
        let kept: std::collections::HashSet<&str> =
            self.messages.iter().map(|x| x.ts.as_str()).collect();
        let anchored = |a: &Option<String>| a.as_deref().is_some_and(|a| kept.contains(a));
        self.tools.blocks.retain(|b| anchored(&b.anchor));
        self.thoughts.settled.retain(|b| anchored(&b.anchor));
        if let Some(i) = self.rewind.marks.iter().position(|x| x.ts == m.ts) {
            self.rewind.marks.truncate(i);
        }
        (self.streaming, self.queued) = Default::default();
        (self.find, self.palette, self.mention) = (None, None, None);
        (self.checklist, self.scroll, self.caret_back) = (Vec::new(), 0, 0);
        self.input = m.text.clone();
        let back = match files {
            0 => String::new(),
            n => format!(
                " \u{2014} {} back",
                crate::wording::count(n as usize, "file")
            ),
        };
        self.push_note(format!(
            "rewound to \u{201c}{}\u{201d}{back}",
            shown(&m.text)
        ));
    }
}
