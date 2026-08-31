//! The palette remembers what you actually run.
//!
//! Among commands that match a query equally well, `cmddefs`'s declaration
//! order decided which came first — an order that means something to whoever
//! last edited that file and nothing at all to the person typing. Type `/` on
//! an empty bar and the list opened on whatever happens to be at the top of
//! the table, every time, no matter that you have run `/gradient` forty times
//! and `/clearlog` never.
//!
//! So the palette keeps a most-recently-used list and uses it as the tie-break.
//! The rule that keeps it predictable is that **recency reorders within a
//! match-quality band and never across one**: a prefix match still beats a
//! fuzzy match, always, so typing `/de` can never float something that does
//! not begin with `de` above something that does. A learned list that can
//! reorder the *kind* of match stops being a list you can aim at.
//!
//! Persisted in the config, so it survives a restart — a shortcut that resets
//! every launch is not one. Capped, because past a handful the tail is not
//! recency any more, it is just the whole table in a different arbitrary
//! order.
//!
//! A process-global, published at config load and after each run, for the same
//! reason [`crate::modelrecents`] is one: `suggest::matches` is a free
//! function on a hot input path with no config handle to thread through.
use std::sync::Mutex;

/// How many commands are remembered.
///
/// Ten is about a screen of palette rows: long enough that a working habit is
/// covered, short enough that the list is still visibly *yours* rather than a
/// slow-moving copy of the whole table.
pub(crate) const MAX: usize = 10;

static RECENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Publish the persisted list (most recent first). Called at config load.
pub(crate) fn set(list: Vec<String>) {
    if let Ok(mut g) = RECENTS.lock() {
        *g = list;
        g.truncate(MAX);
    }
}

/// The list as it stands, most recent first.
pub(crate) fn now() -> Vec<String> {
    RECENTS.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Move `name` (with its leading slash) to the front, and return the new list
/// for the caller to persist. A repeat is a move, not a second entry.
pub(crate) fn record(name: &str) -> Vec<String> {
    let mut list = now();
    list.retain(|c| c != name);
    list.insert(0, name.to_string());
    list.truncate(MAX);
    set(list.clone());
    list
}

/// Where `name` sits in the list — lower is more recent. Anything unrun sorts
/// after everything run, which is what makes this a tie-break rather than a
/// filter.
pub(crate) fn rank_of(list: &[String], name: &str) -> usize {
    list.iter().position(|c| c == name).unwrap_or(usize::MAX)
}

#[cfg(test)]
#[path = "cmdrecents_tests.rs"]
mod tests;
