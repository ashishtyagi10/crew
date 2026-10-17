//! The head of a turn's receipt: what changed, by how much, and how to undo
//! it, on the same card as the diff.
//!
//! A turn that changed files used to leave a pile of notes under its answer:
//! `task #1: 1 file changed…`, then the fenced diff as a card of its own,
//! then the language servers' verdict, the project's check and the pane's
//! own "checked" line, each with its own header and gap. Now the files and
//! the diff go out as ONE note the moment the diff is ready (the pane folds
//! the diff under the head), and the pane files the later notes of the same
//! turn into it (`chatreceipt`).

/// `task #{id}: 1 file changed (+12 −3): ~ src/lib.rs …` — the summary line
/// with the turn's line totals, when it added or removed any.
pub(crate) fn headline(id: u64, summary: &str, (add, del): (usize, usize)) -> String {
    let counted = match summary.split_once(": ") {
        Some((head, rest)) if add + del > 0 => {
            format!("{head} (+{add} \u{2212}{del}): {rest}")
        }
        _ => summary.to_string(),
    };
    format!("task #{id}: {counted}")
}

/// Lines added and removed in `patch`, its file headers not counted.
pub(crate) fn totals(patch: &str) -> (usize, usize) {
    patch.lines().fold((0, 0), |(add, del), l| {
        if l.starts_with('+') && !l.starts_with("+++") {
            (add + 1, del)
        } else if l.starts_with('-') && !l.starts_with("---") {
            (add, del + 1)
        } else {
            (add, del)
        }
    })
}

#[cfg(test)]
#[path = "receipt_tests.rs"]
mod tests;
