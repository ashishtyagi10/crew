//! Agent smith's card (phase 3, Tab on the card): its answer is a JSON
//! object between `<<<TODO` and `TODO>>>`, and every line of it passes the
//! checks crew's own guesses do before it may stand on the card. A project
//! from your list, an assignee on the thread and not you, a due date that
//! `/todo` reads whole as of the newest mail; anything else is dropped, as
//! the mail smith read is a stranger's, and so is what it echoes.
use serde_json::Value;

use super::super::super::api::Mail;
use super::super::super::Reading;
use super::super::Guess;
use super::{due_text, sent, who, Known};

pub(crate) const OPEN: &str = "<<<TODO";
pub(crate) const CLOSE: &str = "TODO>>>";
/// The longest title smith may write, in chars.
const TITLE_MOST: usize = 120;

/// The card's four lines from smith's `answer`: each the line it filled and
/// crew accepts, or `None` where it said nothing crew can stand behind.
/// `None` for an answer with no card in it.
pub(crate) fn check(answer: &str, r: &Reading, k: &Known) -> Option<[Option<Guess>; 4]> {
    let start = answer.rfind(OPEN)? + OPEN.len();
    let end = start + answer[start..].find(CLOSE)?;
    let v: Value = serde_json::from_str(answer[start..end].trim()).ok()?;
    let field = |k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let why = field("why").chars().take(80).collect::<String>();
    let said = |text: String| Guess {
        why: match why.is_empty() {
            true => "agent smith".into(),
            false => format!("agent smith \u{b7} {why}"),
        },
        text,
    };
    let mails = r.mails.as_deref().unwrap_or_default();
    let title = field("title").replace(['\n', '\r'], " ");
    let title =
        (title.chars().count() >= 3 && title.chars().count() <= TITLE_MOST).then_some(title);
    let project = known_project(&field("project"), k);
    let assignee = who::named(mails, &field("assignee"), k);
    let due = read_due(&field("due"), mails);
    Some([title, project, assignee, due].map(|f| f.map(said)))
}

/// `name` as one of your projects, in its own spelling.
fn known_project(name: &str, k: &Known) -> Option<String> {
    let name = name.trim_start_matches('@');
    k.projects
        .iter()
        .chain(&k.folders)
        .find(|p| !name.is_empty() && p.eq_ignore_ascii_case(name))
        .cloned()
}

/// A due smith wrote, read whole as `/todo` reads a date, as of the newest
/// mail: never taken on its word.
fn read_due(text: &str, mails: &[Mail]) -> Option<String> {
    let at = mails
        .last()
        .map_or_else(crate::todopane::duedate::now_local, sent);
    let h = crate::todopane::duedate::find(text, at)?;
    crate::todopane::duedate::strip(text, h.start, h.end)
        .is_empty()
        .then(|| due_text(h.due, h.has_time))
}

#[cfg(test)]
#[path = "todofill_tests.rs"]
mod tests;
