//! One line of todo text read into an item's fields: the due date found and
//! cut out, then the `@project` and `#assignee` tags. The composer's Enter
//! and `crew todo add` both read text this way, so a line typed in a
//! Claude Code pane means exactly what it would in the /todo pane.
use crate::todopane::{duedate, parse};

/// What a line of todo text says.
#[derive(Debug, PartialEq)]
pub(crate) struct Entry {
    pub title: String,
    pub project: Option<String>,
    pub assignee: Option<String>,
    pub due_ms: Option<u64>,
    /// The line named a time of day, not just a date.
    pub has_time: bool,
}

/// `draft` as an item, or `None` when nothing is left once the due and the
/// tags are taken out (a date or a tag alone is not an item). The RAW text
/// is searched, so what the composer tinted is exactly what is cut.
pub(crate) fn parse(draft: &str) -> Option<Entry> {
    let hit = duedate::find(draft, duedate::now_local());
    let rest = match &hit {
        Some(h) => duedate::strip(draft, h.start, h.end),
        None => draft.split_whitespace().collect::<Vec<_>>().join(" "),
    };
    let (title, project, assignee) = parse::extract_tags(&rest);
    if title.is_empty() {
        return None;
    }
    Some(Entry {
        title,
        project,
        assignee,
        due_ms: hit.as_ref().and_then(|h| duedate::to_epoch_ms(h.due)),
        has_time: hit.as_ref().is_some_and(|h| h.has_time),
    })
}

#[cfg(test)]
#[path = "todoentry_tests.rs"]
mod tests;
