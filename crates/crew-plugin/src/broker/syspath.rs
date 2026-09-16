//! What to say when a path is not there.
//!
//! WHY: `read /src/main.rs: No such file or directory (os error 2)` is true
//! and useless. The model's next move is another guess, and the guess after
//! that, because nothing in the error narrows anything down — the one fact
//! that would (what IS in that directory) is sitting right there on the disk
//! and costs a `read_dir` to find out.
//!
//! `sys:edit` proved the shape in 0.22.47: a miss that says WHICH KIND of
//! miss it was turns three blind retries into one corrected call. This is the
//! same idea for the commonest miss of all, and it is what every tool worth
//! copying does — a shell completes, a compiler suggests, and neither makes
//! you guess twice at a name it can see.
//!
//! Two shapes of wrongness, because the fix differs. A file missing from a
//! directory that EXISTS is a misspelling: name the nearest entries. A
//! directory that is itself missing is a wrong branch: say where the path
//! stopped being real and what was there instead.
use std::path::Path;

use crew_hive::tools::near::nearest;

/// Entries listed when nothing is near enough to suggest. Enough to orient,
/// short enough to stay one line.
const LISTED: usize = 6;
/// Chars of directory path shown before it is cut to its last two components.
const DIR_SHOWN: usize = 40;

/// One clause to add to a not-found error, or `None` when there is nothing
/// useful to say — in which case the caller's own message stands alone,
/// which is better than a sentence that narrows nothing.
pub(crate) fn hint(path: &str) -> Option<String> {
    let p = Path::new(path);
    // The deepest ancestor that is really there, and the first component that
    // is not. Walking up rather than testing the parent alone: a model that
    // invented two levels should be told where the path stopped being real.
    let mut missing = p.file_name()?.to_str()?;
    let mut dir = p.parent()?;
    // A bare `main.rs` has an EMPTY parent, not a missing one, and the
    // directory it means is the one the broker is standing in.
    let here = Path::new(".");
    if dir.as_os_str().is_empty() {
        dir = here;
    }
    while !dir.is_dir() {
        missing = dir.file_name()?.to_str()?;
        dir = match dir.parent()? {
            d if d.as_os_str().is_empty() => here,
            d => d,
        };
    }
    // Walked all the way to `/`. Listing the filesystem root narrows nothing
    // and is a sentence about the machine rather than about the mistake.
    dir.parent()?;
    let entries = read_names(dir)?;
    let names: Vec<&str> = entries.iter().map(String::as_str).collect();
    let near = nearest(&names, missing);
    let shown = match near.is_empty() {
        false => format!("did you mean {}?", join(&near)),
        true => format!("it holds {}", listed(&names)),
    };
    Some(format!(
        "{} has no \u{201c}{missing}\u{201d} \u{2014} {shown}",
        where_it_is(dir)
    ))
}

/// The directory, named the way a reader needs it: `.` is a place rather than
/// a punctuation mark, and a deep absolute path is its last two components —
/// the rest is the machine's business and would be most of the line.
fn where_it_is(dir: &Path) -> String {
    if dir == Path::new(".") {
        return "the working directory".to_string();
    }
    let full = dir.display().to_string();
    if full.chars().count() <= DIR_SHOWN {
        return format!("\u{201c}{full}\u{201d}");
    }
    let tail: Vec<String> = dir
        .components()
        .rev()
        .take(2)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    format!(
        "\u{201c}\u{2026}/{}\u{201d}",
        tail.into_iter().rev().collect::<Vec<_>>().join("/")
    )
}

/// The directory's entry names, or `None` when it cannot be read — an
/// unreadable directory is the caller's error to report, not ours to explain.
fn read_names(dir: &Path) -> Option<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        // A dotfile is rarely the thing that was meant and always the thing
        // that fills the line.
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    (!names.is_empty()).then_some(names)
}

/// The first few names, with a count of what did not fit.
fn listed(names: &[&str]) -> String {
    let head = join(&names[..names.len().min(LISTED)]);
    match names.len().saturating_sub(LISTED) {
        0 => head,
        rest => format!("{head} \u{2026} +{rest}"),
    }
}

fn join(names: &[&str]) -> String {
    names
        .iter()
        .map(|n| format!("\u{201c}{n}\u{201d}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `e`, with the hint for `path` when there is one: the one line a caller
/// needs, so every not-found error in the sys surface reads the same way.
pub(crate) fn with_hint(action: &str, path: &str, e: impl std::fmt::Display) -> String {
    match hint(path) {
        Some(h) => format!("{action} {path}: {e} \u{2014} {h}"),
        None => format!("{action} {path}: {e}"),
    }
}

#[cfg(test)]
#[path = "syspath_tests.rs"]
mod tests;
