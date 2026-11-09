//! Callers a signature change left behind, named before the turn is done.
//!
//! A turn that turns `add(a, b)` into `add(a, b, c)` and misses the call in
//! `cli.rs` can still end `✓ checked` when the check never builds `cli.rs`.
//! A staff engineer greps before calling it done; this does: for each name
//! whose definition's parameter count differs between the patch's `-` and
//! `+` lines, every call site in the tree as it is now that still passes the
//! OLD count is named, `! 2 callers of add() not updated: app.rs:4 · cli.rs:2`.
//! The pane files the note into the turn's receipt as a `!` verdict.
//!
//! Matching the old count, not merely the name, is what keeps it honest: a
//! caller already updated passes the new count and is not named, and a call
//! split over lines (whose count one line cannot show) is left out rather
//! than guessed at.
use std::path::Path;

use super::reviewcallers::{call_sites, changed_symbols};

/// Sites named before the rest are counted.
const NAMED: usize = 4;

/// The `!` note for `patch`, or `None` when no signature changed or every
/// caller already fits the new one.
pub(crate) fn note(dir: &Path, patch: &str) -> Option<String> {
    let names = changed_symbols(patch);
    let mut lines = Vec::new();
    for (name, old, new) in arity_changes(patch) {
        if !names.contains(&name) || old == new {
            continue;
        }
        let sites: Vec<String> = call_sites(dir, &name)
            .into_iter()
            .filter(|(_, _, text)| call_args(text, &name) == Some(old))
            .map(|(path, line, _)| format!("{path}:{line}"))
            .collect();
        if sites.is_empty() {
            continue;
        }
        let mut shown = sites[..sites.len().min(NAMED)].join(" \u{b7} ");
        if sites.len() > NAMED {
            shown.push_str(&format!(" \u{b7} +{} more", sites.len() - NAMED));
        }
        let s = if sites.len() == 1 { "" } else { "s" };
        let n = sites.len();
        lines.push(format!("! {n} caller{s} of {name}() not updated: {shown}"));
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// `(name, old call args, new call args)` for each definition the patch both
/// removes and adds on one line — a signature edit — in the order seen.
fn arity_changes(patch: &str) -> Vec<(String, usize, usize)> {
    let (mut old, mut new): (Vec<(String, usize)>, Vec<(String, usize)>) = Default::default();
    for line in patch.lines() {
        if line.starts_with("---") || line.starts_with("+++") {
            continue;
        }
        let side = match line.as_bytes().first() {
            Some(b'-') => &mut old,
            Some(b'+') => &mut new,
            _ => continue,
        };
        if let Some(def) = signature(&line[1..]) {
            side.push(def);
        }
    }
    old.into_iter()
        .filter_map(|(name, o)| {
            let n = new.iter().find(|(m, _)| *m == name)?.1;
            Some((name, o, n))
        })
        .collect()
}

/// The name a definition line declares and how many arguments a CALL of it
/// passes: its parameters, less a leading `self` (a method call passes it
/// before the dot). `None` when the parameter list does not close on the line.
fn signature(line: &str) -> Option<(String, usize)> {
    let (at, kw) = ["fn ", "def ", "function ", "func "]
        .iter()
        .flat_map(|kw| line.match_indices(kw))
        .find(|(i, _)| !line[..*i].ends_with(|c: char| c.is_alphanumeric() || c == '_'))?;
    let rest = line[at + kw.len()..].trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let rest = rest[name.len()..].trim_start();
    let rest = match rest.strip_prefix('<') {
        Some(generics) => &generics[generics.find(">(")? + 1..],
        None => rest,
    };
    let params = args(rest.strip_prefix('(')?, true)?;
    let selfish = params.first().is_some_and(|p| {
        let bare = p.split(':').next().unwrap_or("").trim();
        bare.rsplit([' ', '&']).next() == Some("self")
    });
    (!name.is_empty()).then(|| (name, params.len() - usize::from(selfish)))
}

/// How many arguments the first call of `name` on `text` passes; `None`
/// when `text` has no `name(` or the call does not close on this line.
fn call_args(text: &str, name: &str) -> Option<usize> {
    text.match_indices(name).find_map(|(i, _)| {
        let before = text[..i].chars().next_back();
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            return None;
        }
        let rest = &text[i + name.len()..];
        let rest = match rest.strip_prefix("::<") {
            Some(t) => &t[t.find(">(")? + 1..],
            None => rest,
        };
        Some(args(rest.strip_prefix('(')?, false)?.len())
    })
}

/// The top-level, comma-separated items of a list whose `(` has just been
/// read, up to its `)`; `None` when it does not close. `angles` counts `<…>`
/// as nesting — in a signature, `HashMap<K, V>` is one parameter; in a call,
/// `<` is as likely a comparison.
fn args(s: &str, angles: bool) -> Option<Vec<String>> {
    let (mut depth, mut out, mut cur) = (0usize, Vec::new(), String::new());
    let mut quoted = false;
    let mut prev = ' ';
    for c in s.chars() {
        match c {
            '"' if prev != '\\' => quoted = !quoted,
            _ if quoted => {}
            '(' | '[' | '{' => depth += 1,
            '<' if angles => depth += 1,
            '>' if angles && prev != '-' => depth = depth.saturating_sub(1),
            ')' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                let kept: Vec<String> = out
                    .into_iter()
                    .map(|a| a.trim().to_string())
                    .filter(|a| !a.is_empty())
                    .collect();
                return Some(kept);
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
                prev = c;
                continue;
            }
            _ => {}
        }
        cur.push(c);
        prev = c;
    }
    None
}

#[cfg(test)]
#[path = "stalecallers_tests.rs"]
pub(crate) mod tests;
