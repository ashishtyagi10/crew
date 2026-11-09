//! The sync's one decision, with no GitHub and no disk in sight: given the
//! todos, the repo each `@project` lives in and what GitHub says about the
//! issues already filed, what to write where.
//!
//! An open `@project` todo whose project is a GitHub checkout and that has
//! no issue gets one. A todo with an issue remembers the title and state
//! both sides last agreed ([`TodoIssue`]); a side that differs from that has
//! changed, and the change goes to the other side. When both changed the
//! same field, crew's word stands (the Reminders sync's rule). A done todo
//! whose issue is closed is settled and is not asked about again.
use std::collections::HashMap;

use crate::todopane::item::{TodoIssue, TodoItem};

/// An issue as GitHub has it now.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Remote {
    pub title: String,
    pub open: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    /// File this todo as an issue in `repo`.
    File {
        todo: u64,
        repo: String,
        title: String,
    },
    /// Bring the todo and its issue to `title` / `open`. `push_*` is what
    /// GitHub needs told, `pull_*` what the todo does; the agreement is
    /// written once both sides say it.
    Sync {
        todo: u64,
        issue: TodoIssue,
        /// The todo's title and done as the plan saw them: the crew half is
        /// written only over a todo nobody touched since.
        was: (String, bool),
        title: String,
        open: bool,
        push_title: bool,
        push_open: bool,
    },
}

/// Whether the issue of `it` needs asking about: anything but a done todo
/// whose issue both sides agree is closed.
pub(crate) fn live(it: &TodoItem) -> bool {
    it.issue
        .as_ref()
        .is_some_and(|i| !it.done || i.open || i.title != it.title)
}

/// What to do. `repo_of` names the `owner/name` a project's checkout
/// pushes to (`None`: not a GitHub checkout, or not found); `remote` holds
/// the issues GitHub was asked about, by todo id.
pub(crate) fn plan(
    items: &[TodoItem],
    repo_of: &mut dyn FnMut(&str) -> Option<String>,
    remote: &HashMap<u64, Remote>,
) -> Vec<Step> {
    let mut steps = Vec::new();
    for it in items {
        match &it.issue {
            None if !it.done => {
                let repo = it.project.as_deref().and_then(&mut *repo_of);
                if let Some(repo) = repo {
                    steps.push(Step::File {
                        todo: it.id,
                        repo,
                        title: it.title.clone(),
                    });
                }
            }
            None => {}
            Some(i) => {
                if let Some(r) = remote.get(&it.id) {
                    steps.extend(sync(it, i, r));
                }
            }
        }
    }
    steps
}

/// The side that changed wins; crew's word when both did.
fn pick<T>(crew: bool, gh: bool, mine: T, theirs: T, agreed: T) -> T {
    match (crew, gh) {
        (true, _) => mine,
        (false, true) => theirs,
        _ => agreed,
    }
}

fn sync(it: &TodoItem, last: &TodoIssue, r: &Remote) -> Option<Step> {
    let title = pick(
        it.title != last.title,
        r.title != last.title,
        &it.title,
        &r.title,
        &last.title,
    )
    .clone();
    let open = pick(
        it.done == last.open,
        r.open != last.open,
        !it.done,
        r.open,
        last.open,
    );
    let same = title == it.title && title == r.title && title == last.title;
    if same && open != it.done && open == r.open && open == last.open {
        return None;
    }
    Some(Step::Sync {
        todo: it.id,
        issue: last.clone(),
        was: (it.title.clone(), it.done),
        push_title: title != r.title,
        push_open: open != r.open,
        title,
        open,
    })
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
