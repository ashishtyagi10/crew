//! What crew writes on the todo card before you change it (`todocard`).
use super::Guess;
use crate::todopane::duedate;

/// A due date as the due line holds it: a form that reads back the same
/// whenever it is read (`2026-10-24`, `2026-10-24 17:00`).
pub(crate) fn due_text(h: &duedate::DueHit) -> String {
    let form = if h.has_time {
        "%Y-%m-%d %H:%M"
    } else {
        "%Y-%m-%d"
    };
    h.due.format(form).to_string()
}

/// What crew writes on the card: the title, and the due date the newest mail
/// states, read as of the day it was sent ("by Friday" in Monday's mail is
/// that Friday, whenever `t` is pressed).
pub(super) fn guess(r: &super::super::Reading) -> [Guess; 4] {
    let title = Guess {
        text: super::super::smith::todo_title(r),
        why: String::new(),
    };
    let newest = r.mails.as_deref().and_then(|m| m.last());
    let due = newest
        .and_then(|m| {
            // A date crew cannot read: as of now, as `t` always read it.
            let sent = super::super::days::sent(&m.date);
            let (h, said) =
                super::super::smith::deadline(&m.text, sent.unwrap_or_else(duedate::now_local))?;
            let who = super::super::wire::sender_name(&m.from);
            let of = sent.map_or(String::new(), |d| d.format(" of %b %-d").to_string());
            Some(Guess {
                text: due_text(&h),
                why: format!(
                    "{} \u{b7} \u{201c}{said}\u{201d} in {who}\u{2019}s mail{of}",
                    h.due.format("%a %b %-d")
                ),
            })
        })
        .unwrap_or_default();
    [title, Guess::default(), Guess::default(), due]
}
