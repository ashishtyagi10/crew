//! Rewind, pane side: Ctrl+B on an idle, empty composer lists the messages
//! you sent that started a task, newest first. Enter on one asks the broker
//! what going back would cost (`broker::rewind`), a one-line confirmation
//! names it, and a yes puts the files back, forgets the conversation after
//! that message, and puts its text in the composer to edit and send again.
//!
//! WHY not Esc-Esc, Claude Code's gesture: Esc here already means interrupt
//! while busy and CLOSE THE PANE once idle, so the first press of a double
//! would close it. Ctrl+B ("back") is free, and is the same chord off a Mac.
//!
//! WHY the transcript is cut, not dimmed: dimmed history still reads as
//! context, and the model no longer has it. The cut transcript is exactly
//! what the broker's thread now holds, and a one-line note says where it went.
//! A message is bound to its task by the label the broker announces
//! (`PluginEvent::Task`), so slash commands answered inline never appear.

use crate::chat::ChatPane;
use crate::chatkeys::{ChatAction, ChatInput};
use crate::popupplace::Popup;
use crate::suggest::MenuItem;

/// Sent messages remembered; older ones fall off the front.
const MARKS_MAX: usize = 100;
/// Chars of a message shown in the list and the confirmation.
pub(crate) const SHOWN: usize = 44;
/// How far the broker's task label reaches into a message (`stdio::send`).
pub(crate) const LABEL: usize = 40;

/// A message the user sent, and the task it started once the broker says so.
#[derive(Debug, Clone)]
pub(crate) struct Mark {
    pub ts: String,
    pub text: String,
    pub task: Option<u64>,
}

/// The confirmation on screen: what going back to `task` would change.
#[derive(Debug, Clone)]
pub(crate) struct Ask {
    pub task: u64,
    pub files: u32,
    pub outside: bool,
}

#[derive(Debug, Default)]
pub(crate) struct Rewind {
    pub marks: Vec<Mark>,
    /// The open list's selection (an index into [`Rewind::picks`]).
    pub sel: Option<usize>,
    pub ask: Option<Ask>,
    /// The message picked, until the broker's answer comes back.
    pub want: Option<Mark>,
}

impl Rewind {
    /// The messages that started a task, newest first.
    pub(crate) fn picks(&self) -> Vec<&Mark> {
        self.marks
            .iter()
            .rev()
            .filter(|m| m.task.is_some())
            .collect()
    }
}

/// One line of `text`, clipped.
pub(crate) fn shown(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    match line.char_indices().nth(SHOWN) {
        Some((i, _)) => format!("{}\u{2026}", &line[..i]),
        None => line.to_string(),
    }
}

/// The open list or confirmation as a pop-up above the composer.
pub(crate) fn card(r: &Rewind, cols: u16) -> Option<Popup> {
    if let (Some(a), Some(m)) = (&r.ask, &r.want) {
        let out = if a.outside {
            ", some edited outside crew"
        } else {
            ""
        };
        let item = MenuItem {
            label: format!("rewind to \u{201c}{}\u{201d}?", shown(&m.text)),
            desc: format!(
                "{} go back{out} \u{b7} Enter rewinds \u{b7} Esc keeps",
                crate::wording::count(a.files as usize, "file"),
            ),
            ..Default::default()
        };
        return Some(crate::cmdmenu::popup("rewind", &[item], 0, cols));
    }
    let sel = r.sel?;
    let now = crate::chattime::unix_now_ms();
    let items: Vec<MenuItem> = r
        .picks()
        .iter()
        .map(|m| MenuItem {
            label: shown(&m.text),
            desc: crate::chattime::rel_time(&m.ts, now).unwrap_or_default(),
            ..Default::default()
        })
        .collect();
    Some(crate::cmdmenu::popup(
        "rewind to\u{2026}",
        &items,
        sel,
        cols,
    ))
}

impl ChatPane {
    /// Remember a message the user sent (`push_capped` calls this).
    pub(crate) fn note_sent(&mut self, m: &crate::chatlayout::Message) {
        if m.sender != "user" {
            return;
        }
        let marks = &mut self.rewind.marks;
        marks.push(Mark {
            ts: m.ts.clone(),
            text: m.text.clone(),
            task: None,
        });
        let over = marks.len().saturating_sub(MARKS_MAX);
        marks.drain(..over);
    }

    /// Ctrl+B and the keys of the list and the confirmation, which are modal.
    pub(crate) fn rewind_key(&mut self, k: &ChatInput) -> Option<Option<ChatAction>> {
        if let Some(a) = self.rewind.ask.clone() {
            match k {
                ChatInput::Enter => self.rewind_send(a.task, true),
                ChatInput::Close => self.rewind.want = None,
                _ => return Some(None),
            }
            self.rewind.ask = None;
        } else if let Some(sel) = self.rewind.sel {
            let n = self.rewind.picks().len();
            match k {
                ChatInput::Up => self.rewind.sel = Some(sel.saturating_sub(1)),
                ChatInput::Down => self.rewind.sel = Some((sel + 1).min(n.saturating_sub(1))),
                ChatInput::Enter => self.rewind_pick(sel),
                ChatInput::Close => self.rewind.sel = None,
                _ => {}
            }
        } else if matches!(k, ChatInput::Rewind) {
            self.rewind_open();
        } else {
            return None;
        }
        Some(None)
    }

    fn rewind_open(&mut self) {
        if !self.input.is_empty() || !self.connected {
            return;
        }
        if self.is_busy() {
            return self.push_note("rewind: stop it first (/stop)".into());
        }
        if self.rewind.picks().is_empty() {
            return self.push_note("rewind: no message of yours has started a task yet".into());
        }
        self.rewind.sel = Some(0);
    }

    fn rewind_pick(&mut self, sel: usize) {
        self.rewind.sel = None;
        let Some(m) = self.rewind.picks().get(sel).map(|m| (*m).clone()) else {
            return;
        };
        if self.is_busy() {
            return self.push_note("rewind: stop it first (/stop)".into());
        }
        if let Some(task) = m.task {
            self.rewind.want = Some(m);
            self.rewind_send(task, false);
        }
    }
}

#[cfg(test)]
#[path = "chatrewind_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatrewindfix_tests.rs"]
mod fix;
#[cfg(test)]
#[path = "chatrewindkeys_tests.rs"]
mod keys_tests;
