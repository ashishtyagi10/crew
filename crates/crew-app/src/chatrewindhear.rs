//! Rewind, pane side, part two: what the broker's events do — a task start
//! binds to the message that made it, an answer to our ask opens the
//! confirmation, and the final answer cuts the transcript (`chatrewind`).
use crew_plugin::{PluginCommand, PluginEvent};

use crate::chat::ChatPane;
use crate::chatrewind::shown;
use crate::chatrewindmarks::Ask;

impl ChatPane {
    /// Every broker event, for the two that matter here: a task start binds
    /// to the send that carried its number, and an answer to our ask.
    pub(crate) fn rewind_hears(&mut self, ev: &PluginEvent) {
        match ev {
            PluginEvent::Task {
                id,
                seq,
                running: true,
                ..
            } => self.rewind_bind(*seq, *id),
            PluginEvent::Rewind { task, .. }
                if self
                    .rewind
                    .want
                    .as_ref()
                    .is_some_and(|w| w.task == Some(*task)) =>
            {
                self.rewind_answer(ev);
            }
            _ => {}
        }
    }

    fn rewind_answer(&mut self, ev: &PluginEvent) {
        let PluginEvent::Rewind {
            task,
            files,
            names,
            yours,
            outside,
            unsure,
            stamp,
            pin,
            applied,
            error,
        } = ev
        else {
            return;
        };
        if !error.is_empty() {
            self.rewind.want = None;
            return self.push_note(format!("rewind: {error}"));
        }
        if *applied {
            return self.rewind_done(*files, *pin);
        }
        if *files == 0 {
            return self.rewind_send(*task, (true, stamp)); // the conversation alone: no file step
        }
        self.rewind.ask = Some(Ask {
            task: *task,
            files: *files,
            names: names.clone(),
            yours: yours.clone(),
            outside: *outside,
            unsure: *unsure,
            stamp: stamp.clone(),
        });
    }

    /// Ask the broker (`confirm` false) or tell it yes to the list `seen`.
    pub(crate) fn rewind_send(&mut self, task: u64, (confirm, seen): (bool, &str)) {
        let seen = seen.to_string();
        let _ = self.plugin.send(&PluginCommand::Rewind {
            task,
            confirm,
            seen,
        });
    }

    /// Cut the transcript before the picked message and hand its text back.
    /// The queue is left alone: a rewind is refused while one waits.
    fn rewind_done(&mut self, files: u32, pin: u32) {
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
        self.streaming = Default::default();
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
        // Where the tree it replaced is kept: `undo #N` counts checkpoints
        // newest first, and this one is the newest until a task makes another.
        let kept = match pin {
            0 => String::new(),
            n => format!(
                "; your files before this are pinned: say \u{201c}undo #{n}\u{201d} (a new task moves it down one)"
            ),
        };
        self.push_note(format!(
            "rewound to \u{201c}{}\u{201d}{back}{kept}",
            shown(&m.text)
        ));
    }
}
