//! Enter on the todo card (`todocard`): the row as the card says it, into
//! `/todo`, the tags as `/todo` keeps them and the due line read whole.
use super::super::InboxPane;
use super::Line;
use crate::todopane::duedate;

impl InboxPane {
    /// Enter: the row as the card says it, into `/todo`.
    pub(super) fn add_todo(&mut self) {
        let Some(c) = self.todo.as_ref() else {
            return;
        };
        let title = c.text(Line::Title).to_string();
        if title.is_empty() {
            self.note = "a todo needs a title".into();
            return;
        }
        let due = match c.text(Line::Due) {
            "" => None,
            t => match due_of(t) {
                Some(h) => Some(h),
                None => {
                    self.note = format!(
                        "\u{201c}{t}\u{201d} is not a date /todo reads (fri, oct 24, in 2 days, 2026-10-24)"
                    );
                    return;
                }
            },
        };
        let (project, assignee) = (tag(c.text(Line::Project)), tag(c.text(Line::Who)));
        let now = crate::chattime::unix_now_ms();
        let item = crate::todopane::item::TodoItem {
            title: title.clone(),
            project: project.clone(),
            assignee: assignee.clone(),
            due_ms: due.as_ref().and_then(|h| duedate::to_epoch_ms(h.due)),
            due_has_time: due.as_ref().is_some_and(|h| h.has_time),
            created_ms: now,
            ..Default::default()
        };
        crate::todopane::store::mutate(|items| {
            let id = items.iter().map(|i| i.id).max().unwrap_or(0).max(now - 1) + 1;
            items.push(crate::todopane::item::TodoItem { id, ..item });
        });
        self.todo = None;
        let tags: String = [
            project.map(|p| format!(" @{p}")),
            assignee.map(|a| format!(" #{a}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        let when = due.map_or(String::new(), |h| {
            format!(" (due {})", h.due.format("%a %b %-d"))
        });
        self.note = format!("added to /todo: {title}{tags}{when}");
    }
}

/// A due line, read whole: a date `/todo` reads and nothing else ("by" and
/// "on" before it are part of it), as of now, since it was typed now.
pub(crate) fn due_of(text: &str) -> Option<duedate::DueHit> {
    let h = duedate::find(text, duedate::now_local())?;
    duedate::strip(text, h.start, h.end).is_empty().then_some(h)
}

/// A project or assignee line as `/todo` keeps a tag: no `@` or `#`, one
/// word; none when empty.
fn tag(text: &str) -> Option<String> {
    let t = text.trim_start_matches(['@', '#']);
    let t = t.split_whitespace().collect::<Vec<_>>().join("-");
    (!t.is_empty()).then_some(t)
}
