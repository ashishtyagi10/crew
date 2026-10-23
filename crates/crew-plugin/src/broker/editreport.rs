//! `sys:edit`'s one-line report: which line, and whether the file grew or
//! shrank. A byte count alone never told anyone whether the right thing
//! happened. Moved out of `sysedit` when the edit learned to pick a match by
//! line, which is why the report is handed the places the first edit landed
//! (`ats`) rather than finding them again: with two copies of `old`, finding
//! would name the first copy's line for an edit made at the second.
use super::sysedit::Swap;
use std::path::Path;

/// The report for `swaps` made to `before`, the first of them at `ats`.
pub(super) fn report_all(path: &str, before: &str, swaps: &[Swap], ats: &[usize]) -> String {
    let [one] = swaps else {
        let lines: isize = swaps
            .iter()
            .map(|s| s.new.lines().count() as isize - s.old.lines().count() as isize)
            .sum();
        let delta = match lines {
            0 => String::new(),
            d if d > 0 => format!(", +{d} lines"),
            d => format!(", {d} lines"),
        };
        return format!("edited {} in {} places{delta}", name(path), swaps.len());
    };
    match ats.len() {
        n if n > 1 => format!("edited {} in {n} places (every one)", name(path)),
        _ => report(path, before, ats.first().copied().unwrap_or(0), one),
    }
}

fn report(path: &str, before: &str, at: usize, one: &Swap) -> String {
    // Newlines BEFORE the match, not lines: a prefix ending in `\n` has
    // finished its last line, and the match starts on the one after it.
    let at = before[..at].matches('\n').count() + 1;
    let name = name(path);
    let (was, now) = (one.old.lines().count(), one.new.lines().count());
    let delta = match now as isize - was as isize {
        0 => format!("{was} line{}", plural(was)),
        d if d > 0 => format!("{was} line{} \u{2192} {now}, +{d}", plural(was)),
        d => format!("{was} line{} \u{2192} {now}, {d}", plural(was)),
    };
    format!("edited {name} at line {at} ({delta})")
}

/// The file's own name: the path is the caller's, the name is the reader's.
fn name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .map_or(path, |n| n.to_str().unwrap_or(path))
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}
