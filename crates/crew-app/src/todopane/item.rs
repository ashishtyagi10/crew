//! The todo item record and the list's display ordering.
use serde::{Deserialize, Serialize};

/// One todo. Persisted in `todos.toml` (see [`super::store`]); every field
/// carries `#[serde(default)]` so a file written by a newer build still loads
/// here — the same forward-compat rule as `usage.jsonl`'s `Entry`. `Default`
/// is that same contract in Rust: a field added later is absent from an old
/// file and absent from an old literal, and both have to mean the same
/// thing.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct TodoItem {
    /// Identity: the epoch-ms creation stamp, bumped past any existing id on
    /// a same-millisecond collision. Stable across edits.
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub done: bool,
    /// When the item was ticked done, epoch ms — set on tick, cleared on
    /// un-tick. `None` on items ticked before v0.17 (the done view groups
    /// them under "earlier") — the serde default IS the migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_ms: Option<u64>,
    /// Free-form `@project` tag, created on first use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Free-form `#assignee` tag: WHO the item is for. A second axis beside
    /// `project`, not a kind of it — a task belongs to a project and to a
    /// person, and a list you run a team from has to filter on either.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Due instant, epoch ms (local wall time at save).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_ms: Option<u64>,
    /// Whether the user typed an explicit time-of-day (a date-only due sits
    /// at [`super::duedate::DEFAULT_HOUR`] and its label hides the clock).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub due_has_time: bool,
    #[serde(default)]
    pub created_ms: u64,
    /// The due toast for this item already fired — persisted, so a restart
    /// doesn't re-toast the backlog.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub notified: bool,
}

/// The list's two independent filters: `@project` and `#assignee`. They are
/// AND-ed, and `None` on an axis means every value — including items that
/// carry no tag on it at all. `Copy`, so the view code passes it down
/// without cloning the names.
#[derive(Default, Clone, Copy)]
pub(crate) struct Filters<'a> {
    pub project: Option<&'a str>,
    pub who: Option<&'a str>,
}

impl Filters<'_> {
    pub(crate) fn matches(&self, it: &TodoItem) -> bool {
        tag_eq(it.project.as_deref(), self.project) && tag_eq(it.assignee.as_deref(), self.who)
    }
}

/// A tag satisfies a filter when it is the same word, ignoring case; an
/// unset filter is satisfied by anything, a set one never by an absent tag.
fn tag_eq(tag: Option<&str>, want: Option<&str>) -> bool {
    match want {
        None => true,
        Some(w) => tag.is_some_and(|t| t.eq_ignore_ascii_case(w)),
    }
}

/// Indices of `items` in display order, honouring the active filters:
/// overdue first (due ascending puts every past due ahead of every
/// future one), then upcoming by due, then undated in creation order. Done
/// items are hidden — they stay in the store (`todos.toml` keeps the
/// history) — unless `show_done`, which sinks them below every open item,
/// newest completion first (`h` in the list is the way back: select one
/// and Space un-dones it). Pure — the whole ordering contract lives here
/// and in [`sort_key`].
pub(crate) fn display_order(items: &[TodoItem], f: Filters, show_done: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len())
        .filter(|&i| (show_done || !items[i].done) && f.matches(&items[i]))
        .collect();
    order.sort_by_key(|&i| sort_key(&items[i]));
    order
}

/// How many items are done and match `filter` — what an EMPTY list needs to
/// know before it calls itself empty. A pane whose every item is ticked has
/// no rows left to draw, and saying "no todos" there hides the finished work
/// (and the history holding it) behind a screen that reads like a fresh pane.
pub(crate) fn done_count(items: &[TodoItem], f: Filters) -> usize {
    items.iter().filter(|it| it.done && f.matches(it)).count()
}

/// Rank 1 = dated (due ascending — overdue lands first for free), 2 =
/// undated (creation order), 3 = done (newest completion first — the one
/// you just ticked is the one you're most likely reaching back for; ticks
/// from before the stamp existed fall back to creation); final tie on
/// creation.
fn sort_key(it: &TodoItem) -> (u8, u64, u64) {
    if it.done {
        return (3, u64::MAX - it.done_ms.unwrap_or(it.created_ms), 0);
    }
    match it.due_ms {
        Some(d) => (1, d, it.created_ms),
        None => (2, it.created_ms, 0),
    }
}

/// Indices of the DONE items in history order for the `/todo done` view,
/// honouring the active filters: stamped ticks newest-first, then
/// every legacy (pre-stamp) tick by creation, newest-first. Legacy ticks
/// must stay contiguous at the tail — they share the one "earlier" day
/// bucket, and the headers assume each bucket is one run.
pub(crate) fn done_order(items: &[TodoItem], f: Filters) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].done && f.matches(&items[i]))
        .collect();
    order.sort_by_key(|&i| match items[i].done_ms {
        Some(d) => (0u8, u64::MAX - d, u64::MAX - items[i].created_ms),
        None => (1, u64::MAX - items[i].created_ms, 0),
    });
    order
}

#[cfg(test)]
#[path = "item_tests.rs"]
mod tests;
