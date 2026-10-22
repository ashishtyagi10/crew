//! The app's half of the inbox's hand-offs to agent smith (`inbox::handoff`):
//! it owns both panes, so it delivers the prompt and collects the reply a
//! draft is waiting for.
use crate::app::CrewApp;
use crate::chatlayout::Message;
use crate::inbox::handoff::{ToSmith, Waiting};
use crate::pane::PaneContent;

/// How long a draft waits for smith's answer before giving up.
const DRAFT_WAIT_MS: u64 = 10 * 60_000;

impl CrewApp {
    /// A prompt to deliver, and a draft to collect once smith's turn settles.
    pub(crate) fn poll_inbox(&mut self) -> bool {
        let Some(i) = self
            .panes
            .iter()
            .position(|p| matches!(p.content, PaneContent::Inbox(_)))
        else {
            return false;
        };
        let send = match &mut self.panes[i].content {
            PaneContent::Inbox(p) => p.to_smith.take(),
            _ => None,
        };
        let delivered = send.map(|s| self.deliver(i, s)).is_some();
        delivered | self.collect_draft(i)
    }

    fn smith_index(&self) -> Option<usize> {
        self.panes
            .iter()
            .position(|p| p.label.as_deref() == Some("crew"))
    }

    /// Into smith's pane (opened when it isn't): its transcript shows the
    /// short line, the broker gets the whole fenced thread — the way an
    /// `@file` mention's body reaches the broker but not the display.
    fn deliver(&mut self, inbox: usize, send: ToSmith) {
        if self.smith_index().is_none() {
            self.spawn_crew_pane();
        }
        let Some(s) = self.smith_index() else { return };
        let PaneContent::Chat(c) = &mut self.panes[s].content else {
            return;
        };
        c.push_capped(Message {
            sender: "user".into(),
            text: send.shown,
            ts: chrono::Local::now().timestamp_millis().to_string(),
            meta: String::new(),
            usage: None,
            expanded: false,
        });
        let since = c.messages.len();
        c.submit_command(send.prompt);
        // A summary is read in smith's pane; a draft lands in Gmail, so the
        // inbox keeps the keys and says when it is saved.
        match (send.draft, &mut self.panes[inbox].content) {
            (true, PaneContent::Inbox(p)) => {
                p.waiting = Some(Waiting {
                    since,
                    at_ms: crate::anim::now_ms(),
                });
                self.focused = inbox;
            }
            _ => self.focused = s,
        }
        self.input.focused = false;
    }

    /// Smith's settled answer to a draft request, read for its body.
    fn collect_draft(&mut self, inbox: usize) -> bool {
        let PaneContent::Inbox(p) = &self.panes[inbox].content else {
            return false;
        };
        let Some((since, at)) = p.waiting.as_ref().map(|w| (w.since, w.at_ms)) else {
            return false;
        };
        let answers: Option<Vec<String>> = match self.smith_index().map(|s| &self.panes[s].content)
        {
            Some(PaneContent::Chat(c)) if c.is_busy() => return false,
            Some(PaneContent::Chat(c)) => Some(
                c.messages
                    .iter()
                    .skip(since)
                    .filter(|m| m.sender != "user")
                    .map(|m| m.text.clone())
                    .collect(),
            ),
            _ => None, // smith's pane was closed
        };
        let late = crate::anim::now_ms().saturating_sub(at) > DRAFT_WAIT_MS;
        let body = match answers {
            Some(a) if a.is_empty() && !late => return false, // the turn has not begun
            Some(a) => a
                .iter()
                .rev()
                .find_map(|t| crate::inbox::smith::draft_body(t)),
            None => None,
        };
        if let PaneContent::Inbox(p) = &mut self.panes[inbox].content {
            p.drafted_by_smith(body);
        }
        true
    }
}

#[cfg(test)]
#[path = "inboxsmith_tests.rs"]
mod tests;
