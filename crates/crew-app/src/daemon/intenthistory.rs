//! What a standing intent has already done — the other half of the log.
//!
//! [`super::intentlog::Watchlist::live`] folds firings AWAY: it answers "what is still
//! standing", so a daily briefing that has fired forty times looks identical to one set this
//! morning, and a firing missed to a shut laptop is announced once on its channel and then
//! unrecoverable. The `Fired` entries carry all of it — when, and how many occurrences the roll
//! stepped over — and nothing read them back. This does.
use std::collections::BTreeMap;

use super::intentlog::{Entry, Watchlist};

/// One intent's firings, folded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Fired {
    /// Times it ran.
    pub count: u64,
    /// When it last ran.
    pub last_ms: u64,
    /// Occurrences it never ran, because the machine was not up for them.
    pub missed: u64,
}

impl Watchlist {
    /// Every id that has ever fired, with its firings folded. Cancelled and standing ones
    /// alike: a cancelled repeat's history is still what it did.
    pub(crate) fn history(&self) -> BTreeMap<String, Fired> {
        let mut out: BTreeMap<String, Fired> = BTreeMap::new();
        for e in self.entries().0 {
            if let Entry::Fired {
                id, at_ms, skipped, ..
            } = e
            {
                let f = out.entry(id).or_default();
                f.count += 1;
                f.last_ms = f.last_ms.max(at_ms);
                f.missed += skipped;
            }
        }
        out
    }
}

/// `fired 40× · last 2h ago · 3 missed` — the missed count only when there is one, since
/// `0 missed` on every row is noise dressed as reassurance.
pub(crate) fn note(f: &Fired, now_ms: u64) -> String {
    let mut s = match f.count {
        1 => "fired once".to_string(),
        n => format!("fired {n}\u{d7}"),
    };
    s.push_str(&format!(
        " \u{b7} last {}",
        crate::toolsrow::ago(f.last_ms, now_ms)
    ));
    if f.missed > 0 {
        s.push_str(&format!(" \u{b7} {} missed", f.missed));
    }
    s
}

#[cfg(test)]
#[path = "intenthistory_tests.rs"]
mod tests;
