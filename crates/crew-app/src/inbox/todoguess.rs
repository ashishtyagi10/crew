//! What crew writes on the todo card before you change it (`todocard`),
//! read from the thread on your computer (goal 2026-10-22-inbox-smart-todo,
//! phase 2): the ask as the title, a project you already have, a person on
//! the thread, and the deadline. A guess crew cannot defend is left blank: a
//! wrong `@project` is worse than none.
use super::super::api::Mail;
use super::super::Reading;
use super::Guess;
use crate::todopane::duedate;

#[path = "todoask.rs"]
mod ask;
#[path = "tododue.rs"]
mod due;
#[path = "todoproject.rs"]
mod project;
#[path = "todowho.rs"]
mod who;

/// What crew knows besides the thread: the projects and people you already
/// tag, the folders projects live in, and whose account this is.
#[derive(Default)]
pub(crate) struct Known {
    /// `@project` tags in `/todo` and the names bound in `projects.toml`.
    pub projects: Vec<String>,
    /// Folders under the roots projects live in (`projectdir`): a project
    /// only on a strong sign, a `[name]` subject or a repository link.
    pub folders: Vec<String>,
    /// `#assignee` tags in `/todo`.
    pub people: Vec<String>,
    /// The signed-in address: never an assignee, and the mails it sent ask
    /// nothing of you.
    pub me: String,
}

impl Known {
    /// What this machine knows. A test reads none of it: the todo list,
    /// `projects.toml` and the folders are the user's, not the test's.
    pub(crate) fn here(me: &str) -> Self {
        let mut k = Known {
            me: me.to_string(),
            ..Known::default()
        };
        if cfg!(test) {
            return k;
        }
        use crate::todopane::projectdir;
        for it in crate::todopane::store::snapshot() {
            k.projects.extend(it.project);
            k.people.extend(it.assignee);
        }
        let bound = projectdir::bound();
        k.projects.extend(bound.keys().cloned());
        for root in projectdir::roots(&bound) {
            let names = std::fs::read_dir(root).into_iter().flatten().flatten();
            k.folders.extend(
                names
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| !n.starts_with('.')),
            );
        }
        for v in [&mut k.projects, &mut k.folders, &mut k.people] {
            v.sort();
            v.dedup();
        }
        k
    }
}

/// A due date as the due line holds it: a form that reads back the same
/// whenever it is read (`2026-10-24`, `2026-10-24 17:00`).
pub(crate) fn due_text(due: chrono::NaiveDateTime, has_time: bool) -> String {
    let form = if has_time {
        "%Y-%m-%d %H:%M"
    } else {
        "%Y-%m-%d"
    };
    due.format(form).to_string()
}

/// The card's four lines: title, project, assignee, due.
pub(super) fn guess(r: &Reading, k: &Known) -> [Guess; 4] {
    let mails = r.mails.as_deref().unwrap_or_default();
    // The ask is in the newest mail someone else sent.
    let theirs = mails
        .iter()
        .rev()
        .find(|m| !mine(m, &k.me))
        .or(mails.last());
    [
        ask::title(r, theirs),
        project::guess(&r.thread.subject, mails, k),
        who::guess(mails, theirs, k),
        due::guess(&r.thread.subject, mails),
    ]
}

/// Whether the signed-in account sent `m`.
fn mine(m: &Mail, me: &str) -> bool {
    !me.is_empty() && super::super::addr::address(&m.from).eq_ignore_ascii_case(me)
}

/// When `m` was sent, in local wall time; now when its date cannot be read.
fn sent(m: &Mail) -> chrono::NaiveDateTime {
    super::super::days::sent(&m.date).unwrap_or_else(duedate::now_local)
}

/// A mail's own words, without the history it quotes; its text when it has
/// no words apart (a plain mail read before `rich`, or a test's).
fn words(m: &Mail) -> &str {
    match m.rich.body.trim().is_empty() {
        true => &m.text,
        false => &m.rich.body,
    }
}

/// `Ann Lee's mail of Oct 20`: where a guess came from.
fn whose(m: &Mail) -> String {
    let who = super::super::wire::sender_name(&m.from);
    match super::super::days::sent(&m.date) {
        Some(d) => format!("{who}\u{2019}s mail of {}", d.format("%b %-d")),
        None => format!("{who}\u{2019}s mail"),
    }
}

#[cfg(test)]
#[path = "todoguess_tests.rs"]
mod tests;
