//! In-pane search: a pure matcher over rendered line text plus a cursor that
//! wraps. Kept out of `keys` so `n`/`N` behaviour is testable without a pane.

/// Indexes of the lines containing `needle`, case-insensitively. An empty
/// needle matches nothing — matching everything would make `n` walk the whole
/// file for no reason.
pub(crate) fn find_matches(lines: &[impl AsRef<str>], needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let needle = needle.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.as_ref().to_lowercase().contains(&needle))
        .map(|(i, _)| i)
        .collect()
}

/// A live search: the needle, its hits, and where `n`/`N` last landed.
pub(crate) struct Search {
    pub needle: String,
    pub hits: Vec<usize>,
    /// Index into `hits`, `None` before the first `n`.
    at: Option<usize>,
    /// True while the user is still typing the needle.
    pub typing: bool,
}

impl Search {
    pub(crate) fn new(needle: String, hits: Vec<usize>) -> Self {
        Self {
            needle,
            hits,
            at: None,
            typing: false,
        }
    }

    /// Forget where `n`/`N` last landed. Called whenever `hits` is replaced
    /// wholesale (a needle edit, or the rendering underneath an unchanged
    /// needle changing shape) — the old index may point past the end of a
    /// shorter list, or at a different hit entirely in a reordered one, so
    /// the next `n` should start fresh rather than trust a stale position.
    pub(crate) fn reset_cursor(&mut self) {
        self.at = None;
    }

    /// The next hit's line, wrapping at the end.
    pub(crate) fn next(&mut self) -> Option<usize> {
        if self.hits.is_empty() {
            return None;
        }
        let i = match self.at {
            None => 0,
            Some(i) => (i + 1) % self.hits.len(),
        };
        self.at = Some(i);
        Some(self.hits[i])
    }

    /// The previous hit's line, wrapping at the start.
    pub(crate) fn prev(&mut self) -> Option<usize> {
        if self.hits.is_empty() {
            return None;
        }
        let i = match self.at {
            None => self.hits.len() - 1,
            Some(0) => self.hits.len() - 1,
            Some(i) => i - 1,
        };
        self.at = Some(i);
        Some(self.hits[i])
    }
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
