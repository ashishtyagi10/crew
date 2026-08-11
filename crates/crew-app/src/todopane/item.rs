//! The todo item record and the list's display ordering.
use serde::{Deserialize, Serialize};

/// One todo. Persisted in `todos.toml` (see [`super::store`]); every field
/// carries `#[serde(default)]` so a file written by a newer build still loads
/// here — the same forward-compat rule as `usage.jsonl`'s `Entry`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct TodoItem {
    /// Identity: the epoch-ms creation stamp, bumped past any existing id on
    /// a same-millisecond collision. Stable across edits.
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub done: bool,
    /// Free-form `@project` tag, created on first use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
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

/// Indices of `items` in display order, honouring an active `@project`
/// filter: overdue first (due ascending puts every past due ahead of every
/// future one), then upcoming by due, then undated in creation order. Done
/// items are hidden — they stay in the store (`todos.toml` keeps the
/// history) — unless `show_done`, which sinks them below every open item,
/// newest completion first (`h` in the list is the way back: select one
/// and Space un-dones it). Pure — the whole ordering contract lives here
/// and in [`sort_key`].
pub(crate) fn display_order(
    items: &[TodoItem],
    filter: Option<&str>,
    show_done: bool,
) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len())
        .filter(|&i| {
            (show_done || !items[i].done)
                && match filter {
                    None => true,
                    Some(f) => items[i]
                        .project
                        .as_deref()
                        .is_some_and(|p| p.eq_ignore_ascii_case(f)),
                }
        })
        .collect();
    order.sort_by_key(|&i| sort_key(&items[i]));
    order
}

/// Rank 1 = dated (due ascending — overdue lands first for free), 2 =
/// undated (creation order), 3 = done (newest completion first — the one
/// you just ticked is the one you're most likely reaching back for);
/// final tie on creation.
fn sort_key(it: &TodoItem) -> (u8, u64, u64) {
    if it.done {
        return (3, u64::MAX - it.created_ms, 0);
    }
    match it.due_ms {
        Some(d) => (1, d, it.created_ms),
        None => (2, it.created_ms, 0),
    }
}

#[cfg(test)]
#[path = "item_tests.rs"]
mod tests;
