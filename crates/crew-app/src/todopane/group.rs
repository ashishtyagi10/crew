//! Grouping the list by `#assignee`: the display order, the bands between
//! the rows, and the roll-up each band carries.
//!
//! A flat list sorted by due date answers "what is next"; running a team
//! asks a different question — "where is everyone" — and that one is a
//! per-person question. So `g` bands the same items under their owner with
//! a live tally beside the name, and the ungrouped list is untouched.

use super::item::{Filters, TodoItem};
use super::TodoPane;

/// Which header bands the list draws between its rows: the done history's
/// day buckets, `#assignee` groups, or nothing. One enum rather than two
/// bools so no view can ask for both — the history is already a log.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Bands {
    None,
    Days,
    People,
}

/// An item's group key: its `#assignee` lowercased, `None` for unassigned.
pub(crate) fn who(it: &TodoItem) -> Option<String> {
    it.assignee.as_ref().map(|a| a.to_lowercase())
}

/// The people carrying at least one item that passes `f`, alphabetically
/// (case-insensitive), keeping each name's first-seen spelling.
///
/// Alphabetical, not by volume: the bands are a place you look every
/// morning, and a list that reorders itself because someone closed two
/// tickets is one you have to re-read instead of scan.
pub(crate) fn people(items: &[TodoItem], f: Filters) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for it in items.iter().filter(|it| f.matches(it)) {
        let Some(a) = &it.assignee else { continue };
        if !names.iter().any(|n| n.eq_ignore_ascii_case(a)) {
            names.push(a.clone());
        }
    }
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// Display order banded by person: the flat order ([`super::item::display_order`])
/// re-sorted by owner, unassigned last. The sort is STABLE, so inside a band
/// the items keep the overdue → due → undated ordering the flat list gives
/// them — a band is a slice of the same list, not a different one.
pub(crate) fn order(items: &[TodoItem], f: Filters, show_done: bool) -> Vec<usize> {
    let names = people(items, f);
    let mut order = super::item::display_order(items, f, show_done);
    order.sort_by_key(|&i| {
        who(&items[i])
            .and_then(|k| names.iter().position(|n| n.eq_ignore_ascii_case(&k)))
            .unwrap_or(names.len())
    });
    order
}

/// Whether display row `di` opens a band — its header row rides on this
/// item, so every height sum stays a per-item sum. THE band truth: the
/// draw, the scroll math and the hit-test all call this one function, so a
/// header can never be counted by one and missed by another.
pub(crate) fn starts(p: &TodoPane, order: &[usize], di: usize) -> bool {
    let at = |di: usize| order.get(di).map(|&i| &p.items[i]);
    let Some(it) = at(di) else { return false };
    let prev = di.checked_sub(1).and_then(at);
    match p.bands() {
        Bands::None => false,
        Bands::Days => {
            let day = super::measure::done_day(it);
            prev.is_none_or(|q| super::measure::done_day(q) != day)
        }
        Bands::People => prev.is_none_or(|q| who(q) != who(it)),
    }
}

/// The roll-up beside a band's name: how many of that person's items are
/// open, how many of those are overdue, and how many they ticked today.
///
/// Counted over the whole store, not the visible rows: done items are
/// hidden by default and "done today" is the half of a standup the list
/// otherwise cannot show.
pub(crate) fn tally(items: &[TodoItem], f: Filters, key: Option<&str>, now_ms: u64) -> String {
    let today = super::duedate::from_epoch_ms(now_ms).map(|d| d.date());
    let mine = items
        .iter()
        .filter(|it| f.matches(it) && who(it).as_deref() == key);
    let (mut open, mut overdue, mut done) = (0, 0, 0);
    for it in mine {
        match (it.done, it.due_ms) {
            (true, _) => {
                done += u32::from(today.is_some() && super::measure::done_day(it) == today)
            }
            (false, due) => {
                open += 1;
                overdue += u32::from(due.is_some_and(|d| d <= now_ms));
            }
        }
    }
    format!("{open} open \u{b7} {overdue} overdue \u{b7} {done} done today")
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
