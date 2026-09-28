//! Fitting `sys:grep`'s hits and `sys:glob`'s paths to what the agent is
//! shown.
//!
//! The agent sees ~6,000 chars of a tool result, and 150 hits of up to 200
//! chars each came to far more: the relay's clip kept the first thirty or so,
//! and the count at the end said how many the 150 cap had cut, not how many
//! the clip had, so the agent could not tell how much of the answer it had
//! seen. Fitted here, under [`RUN_FIT`] like a `sys:run` or `sys:git` result,
//! the output stops at a whole hit and its last line says what was left out.
use super::toolclip::RUN_FIT;

/// Bytes kept back from [`RUN_FIT`] for the closing line: two 20-digit counts
/// and its words come to under 150.
const TAIL: usize = 200;

/// Items — hits, or paths — in the order they were found: kept while they fit,
/// only counted after.
#[derive(Default)]
pub(super) struct Fit {
    /// Each kept item's text, and its file's number among the files with any.
    kept: Vec<(String, usize)>,
    /// Bytes of `kept`, each item with the newline after it.
    bytes: usize,
    /// Every item seen, kept or not.
    items: usize,
    /// Files seen with at least one item.
    files: usize,
    /// The file the first item not kept is in, once there is one.
    cut: Option<usize>,
}

impl Fit {
    /// Whether items are still kept. Once one is not, none after it is — the
    /// output is a run from the start, never a hit here and there — so the
    /// caller need not format them.
    pub(super) fn keeping(&self) -> bool {
        self.cut.is_none()
    }

    /// Nothing found at all.
    pub(super) fn is_empty(&self) -> bool {
        self.items == 0
    }

    /// The items pushed or counted from here on are the next file's.
    pub(super) fn file(&mut self) {
        self.files += 1;
    }

    /// One item: kept if everything kept so far, and it, still fit whole.
    pub(super) fn push(&mut self, text: String) {
        if self.keeping() && self.bytes + text.len() <= RUN_FIT {
            self.bytes += text.len() + 1;
            self.kept.push((text, self.files.saturating_sub(1)));
            self.items += 1;
        } else {
            self.count(1);
        }
    }

    /// `n` items not kept.
    pub(super) fn count(&mut self, n: usize) {
        if n > 0 {
            self.items += n;
            self.cut.get_or_insert(self.files.saturating_sub(1));
        }
    }

    /// The kept items, one per line. When all of them fit that is the whole
    /// output, so a search that always fitted reads as it always has.
    /// Otherwise items come off the end until there is room for
    /// `tail(items left out, files they are in)`, which ends it.
    pub(super) fn finish(mut self, tail: impl Fn(usize, usize) -> String) -> String {
        let Some(mut cut) = self.cut else {
            let kept: Vec<String> = self.kept.into_iter().map(|(t, _)| t).collect();
            return kept.join("\n");
        };
        while self.bytes + TAIL > RUN_FIT {
            let Some((text, file)) = self.kept.pop() else {
                break;
            };
            self.bytes -= text.len() + 1;
            cut = file;
        }
        let left = self.items - self.kept.len();
        let mut out: String = self.kept.into_iter().map(|(t, _)| t + "\n").collect();
        out.push_str(&tail(left, self.files - cut));
        out
    }
}

/// `3 hits`, `1 hit`.
pub(super) fn counted(n: usize, noun: &str) -> String {
    format!("{n} {noun}{}", if n == 1 { "" } else { "s" })
}

#[cfg(test)]
#[path = "sysgrepfit_tests.rs"]
mod tests;
