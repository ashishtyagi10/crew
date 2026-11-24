//! A user's correction is proposed before it is kept.
//!
//! WHY: a correction leads every recall it reaches and reads as an order
//! ("ruled out before"), and it was written the moment the router or the
//! openings caught one — the user never saw it happen, so a "no, rush it"
//! misread as a correction steered later work unseen. Kiro's Memory, Codex
//! and Gemini CLI propose a memory and let the user accept it; so does crew.
//!
//! WHY in memory and not in the graph: a pending node on disk would be one
//! more `Turn` that every reader (`query`, `block`, `ruled`, `/doctor`'s
//! counts) must learn to skip, and an older build replaying the log would
//! recall it as an ordinary turn. Held here it cannot leak into a prompt.
//!
//! One slot. A newer correction replaces an unanswered older one: the line
//! asking about the older one was shown and passed over, and silence is not
//! a yes. A broker restart drops it for the same reason — nothing proposed
//! is ever kept unless the user said so.
use super::node::Kind;
use super::ruled::RULED;
use super::Recall;

/// What waits for the user's word: the correction, and the keys of the
/// files of the turn it corrects, taken NOW — by the time "keep" arrives
/// the turn recorded last is the correcting message itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct Proposed {
    what: String,
    files: Vec<String>,
}

impl Recall {
    /// Propose `what`, and say whether to ask. `false` when recall is off,
    /// when the same words are already pending (asked once is enough), and
    /// when they are already kept — the user agreed to that line once, so
    /// it is sighted again (`record_ruled`'s second hit), as it always was.
    pub(crate) fn propose_ruled(&mut self, what: &str) -> bool {
        let what = what.trim();
        if !self.on || what.is_empty() || self.proposed() == Some(what) {
            return false;
        }
        let key = format!("{RULED}{what}");
        if self.g.find(Kind::Turn, &key).is_some() {
            self.record_ruled(what, true);
            return false;
        }
        self.proposed = Some(Proposed {
            what: what.to_string(),
            files: self.last_files(),
        });
        true
    }

    /// The correction waiting for the user's word, if one is.
    pub(crate) fn proposed(&self) -> Option<&str> {
        self.proposed.as_ref().map(|p| p.what.as_str())
    }

    /// The user kept it: written as any correction is, tied to the files
    /// the proposal saw. Says what was kept.
    pub(crate) fn keep_proposed(&mut self) -> Option<String> {
        let p = self.proposed.take()?;
        self.record_ruled_tied(&p.what, &p.files);
        Some(p.what)
    }

    /// The user forgot it: nothing is written. Says what went.
    pub(crate) fn drop_proposed(&mut self) -> Option<String> {
        self.proposed.take().map(|p| p.what)
    }
}

#[cfg(test)]
#[path = "proposed_tests.rs"]
mod tests;
