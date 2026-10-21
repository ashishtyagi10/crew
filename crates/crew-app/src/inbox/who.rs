//! Who a thread is between, written the way Gmail writes it (goal
//! 2026-10-20-inbox-list-look, item 6): `Ann, Bo, me`, from the `From` of
//! every message, each person once and the newest last, `me` for the account
//! itself. One sender is named in full; several go by first names, then
//! `Ann … Cy`, then the newest alone, as the room runs out.
use super::wire::{sender_name, Thread};
use crate::chatwidth::{clip_w, str_w};

/// One person on the thread: how to tell them apart, and the two ways to
/// write them.
struct Person {
    key: String,
    full: String,
    first: String,
}

/// The address in a `From` header, lowercased: `Ann <ann@x.org>` → `ann@x.org`.
pub(crate) fn address(from: &str) -> String {
    let inner = from.rsplit_once('<').map_or(from, |(_, a)| a);
    inner.trim().trim_end_matches('>').trim().to_lowercase()
}

fn person(from: &str, sent: bool, me: &str) -> Person {
    let key = address(from);
    if sent || (!me.is_empty() && key == me.to_lowercase()) {
        return Person {
            key: "me".into(),
            full: "me".into(),
            first: "me".into(),
        };
    }
    let full = sender_name(from);
    let first = match full.contains('@') {
        true => full.split('@').next().unwrap_or(&full).to_string(),
        false => full.split_whitespace().next().unwrap_or(&full).to_string(),
    };
    Person { key, full, first }
}

/// Everyone on `th`, each once, in the order they last wrote.
fn people(th: &Thread, me: &str) -> Vec<Person> {
    let all: Vec<Person> = match th.senders.is_empty() {
        true => vec![person(&th.from, false, me)],
        false => th
            .senders
            .iter()
            .map(|s| person(&s.from, s.sent, me))
            .collect(),
    };
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Person> = all
        .into_iter()
        .rev()
        .filter(|p| seen.insert(p.key.clone()))
        .collect();
    out.reverse();
    out
}

/// The thread's people in at most `room` columns; `all` false names only
/// the newest (the width ladder's third step).
pub(crate) fn names(th: &Thread, me: &str, room: usize, all: bool) -> String {
    let mut ps = people(th, me);
    if !all && ps.len() > 1 {
        ps.drain(..ps.len() - 1);
    }
    let fits = |s: String| (str_w(&s) <= room).then_some(s);
    let tries = match ps.as_slice() {
        [] => vec![String::new()],
        [one] => vec![one.full.clone(), one.first.clone()],
        [first, .., last] => vec![
            ps.iter()
                .map(|p| p.full.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            ps.iter()
                .map(|p| p.first.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            format!("{} \u{2026} {}", first.first, last.first),
            last.first.clone(),
        ],
    };
    let last = tries.last().cloned().unwrap_or_default();
    tries
        .into_iter()
        .find_map(fits)
        .unwrap_or_else(|| clip_w(&last, room))
}

#[cfg(test)]
#[path = "who_tests.rs"]
mod tests;
