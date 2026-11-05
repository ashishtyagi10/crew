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

/// The patch lines under a receipt's head: the fenced diff, fences not
/// counted. 0 for a receipt with no diff (or any other card).
pub(crate) fn diff_lines(text: &str) -> usize {
    let Some(at) = text.find("\n```") else {
        return 0;
    };
    let fenced = text[at + 1..].lines().count();
    let fences = if text.trim_end().ends_with("```") {
        2
    } else {
        1
    };
    fenced.saturating_sub(fences)
}

/// Marks what a fold hid. A receipt's kept rows end on a verdict, and the
/// ` … +N` that rode that line cut its last words off and sat under the
/// head's `(+4 −1)`, where it read as five more lines added. So a receipt's
/// fold gets a row of its own that names it, `▸ diff · 9 lines`; any other
/// card (and the compact view, one line a card) keeps the suffix.
pub(crate) fn mark_fold(
    m: &Message,
    body: &mut Vec<crate::chatbody::CardLine>,
    hidden: usize,
    cols: usize,
    compact: bool,
) {
    let room = cols.saturating_sub(1); // the last column stays margin
    let n = diff_lines(&m.text);
    if compact || n == 0 || !is_receipt(m) {
        if let Some(last) = body.last_mut() {
            crate::chathidden::append_hidden_suffix(last, hidden, room);
        }
        return;
    }
    let muted = crew_theme::theme().text_muted;
    let unit = if n == 1 { "line" } else { "lines" };
    let row = format!(" \u{25b8} diff \u{b7} {n} {unit}");
    body.push(
        row.chars()
            .take(room)
            .map(|c| crate::chatbody::plain(c, muted, false))
            .collect(),
    );
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
