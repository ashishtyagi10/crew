//! The palette's answer to a folded spelling.
//!
//! Fifteen appearance commands live under `/look` now (`/theme` is `/look
//! theme`), and `/clearall` is `/clear all` — every old spelling still runs
//! (`verbs::FOLDED`), but no palette row offers it. So typing what you
//! remember drew rows that only happened to hold its letters: `/th` offered
//! `/smith`, `/watching` and `/batch`, and nothing about themes (2026-10-09).
//! Now a query that begins an old spelling is offered its new home, right
//! after the commands that begin with it.
use crate::suggest::MenuItem;
use crate::verbs::{nested, VERBS};

/// The shortest query that reaches for a folded spelling: one letter would
/// offer half the `/look` subjects on every keystroke.
const MIN_QUERY: usize = 2;

/// `(old spelling, verb, subject, what it does)` for every folded subject:
/// a `/look` subject was its own command; any other verb's was the verb's
/// name with the subject glued on (`/clear` + `all` = `/clearall`).
fn spellings() -> impl Iterator<Item = (String, &'static str, &'static str, &'static str)> {
    VERBS.iter().flat_map(|v| {
        v.subjects.iter().map(move |&(s, desc)| {
            let old = match v.name {
                "/look" => s.to_string(),
                verb => format!("{}{s}", &verb[1..]),
            };
            (old, v.name, s, desc)
        })
    })
}

/// The rows `text` reaches by an old spelling — none once it has a space,
/// or while it is still just the start of the verb's own name.
pub(crate) fn rows(text: &str) -> Vec<MenuItem> {
    let Some(q) = text.strip_prefix('/').map(str::to_lowercase) else {
        return Vec::new();
    };
    if q.chars().count() < MIN_QUERY || q.contains(' ') {
        return Vec::new();
    }
    spellings()
        .filter(|(old, verb, _, _)| old.starts_with(&q) && !verb[1..].starts_with(&q))
        .map(|(_, verb, s, desc)| {
            let label = format!("{verb} {s}");
            MenuItem {
                fill: crate::verbs::fill(verb, s),
                submit: !nested(verb, s),
                hit: hits(&label, verb, q.chars().count()),
                label,
                desc: desc.to_string(),
                ..Default::default()
            }
        })
        .collect()
}

/// The `n` characters of `label` the query spelled: the subject's for a
/// `/look` row (`/theme` was the subject alone), the verb's and then the
/// subject's for the rest (`/clearall` is both, glued).
fn hits(label: &str, verb: &str, n: usize) -> Vec<usize> {
    let from = if verb == "/look" { verb.len() + 1 } else { 1 };
    label
        .char_indices()
        .filter(|&(i, c)| i >= from && c != ' ')
        .map(|(i, _)| i)
        .take(n)
        .collect()
}

/// `rows` (the palette's own matches for `text`, prefix band first) with the
/// folded spellings' rows slotted in after that band.
pub(crate) fn merged(text: &str, mut rows: Vec<MenuItem>) -> Vec<MenuItem> {
    let at = rows
        .iter()
        .take_while(|r| r.label.starts_with(text))
        .count();
    let extra = self::rows(text);
    rows.splice(at..at, extra);
    rows
}

#[cfg(test)]
#[path = "foldrows_tests.rs"]
mod tests;
