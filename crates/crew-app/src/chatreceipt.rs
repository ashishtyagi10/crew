//! A turn's receipt: one card for what a turn that changed files leaves.
//!
//! The broker sends the receipt's head and the diff together, `task #1: 1
//! file changed (+1 −1): ~ src/lib.rs …` over the fenced patch
//! (`broker::receipt`). What smith learns after that, about the same turn —
//! the language servers' verdict, the project's check, the pane's own
//! "checked" line (`chatproof`), the one-time tip — used to land as a card
//! each, header and gap and all: four cards of chrome under one answer. Now
//! each is filed into the receipt, above the diff, so the card reads head,
//! verdicts, then the folded patch.
//!
//! Only while the receipt is the transcript's last message, and only bare
//! `agent smith` notes: an agent's reply, the next question or a repair
//! pass's first answer ends the receipt, and nothing after it is pulled in.
use crate::chat::ChatPane;
use crate::chatlayout::Message;

/// Whether `m` is a receipt: smith's `task #N: …` note.
pub(crate) fn is_receipt(m: &Message) -> bool {
    m.sender == "agent smith" && m.text.starts_with("task #")
}

/// `note` inserted into `receipt` above its diff fence, or at its end when
/// it has none.
pub(crate) fn filed(receipt: &str, note: &str) -> String {
    let fence = receipt.find("\n```").map(|i| i + 1);
    match fence {
        Some(at) => format!("{}{note}\n{}", &receipt[..at], &receipt[at..]),
        None => format!("{receipt}\n{note}"),
    }
}

/// The rows a folded card keeps: a receipt, every row above its diff (the
/// head and what smith filed under it); any other card, its first.
pub(crate) fn kept_rows(m: &Message, cols: usize, view: crate::chatmsgs::View<'_>) -> usize {
    let at = m.text.find("\n```").filter(|_| is_receipt(m));
    let Some(at) = at else { return 1 };
    let head = Message {
        sender: m.sender.clone(),
        text: m.text[..at].to_string(),
        ts: m.ts.clone(),
        meta: m.meta.clone(),
        usage: m.usage,
        expanded: m.expanded,
    };
    crate::chatmsgs::full_body(&head, cols, view).len()
}

impl ChatPane {
    /// File `text` into the receipt it follows; `false` when it is not a
    /// note of that turn, and the caller posts it as a card of its own.
    pub(crate) fn into_receipt(&mut self, sender: &str, text: &str) -> bool {
        if sender != "agent smith" || text.starts_with("task #") {
            return false;
        }
        let Some(last) = self.messages.last_mut().filter(|m| is_receipt(m)) else {
            return false;
        };
        last.text = filed(&last.text, text.trim_end());
        true
    }
}

#[cfg(test)]
#[path = "chatreceipt_tests.rs"]
mod tests;
