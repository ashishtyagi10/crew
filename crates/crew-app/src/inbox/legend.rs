//! What the inbox says about itself outside its rows: the card's legend and
//! the nav's one-line glance.
use super::{InboxPane, Stage};

impl InboxPane {
    /// The card's legend: `inbox · 12 unread`, or the search shown.
    pub(crate) fn legend(&self) -> String {
        match (self.query.trim(), self.unread) {
            ("", Some(n)) if n > 0 => format!("inbox \u{b7} {n} unread"),
            ("", _) => "inbox".into(),
            (q, _) => format!("inbox \u{b7} {q}"),
        }
    }

    /// The nav's one-line glance.
    pub(crate) fn glance(&self) -> String {
        match (&self.stage, self.unread) {
            (Stage::Ready(_), Some(n)) => format!("{n} unread"),
            (Stage::Ready(_), None) => "checking\u{2026}".into(),
            _ => "not signed in".into(),
        }
    }
}
