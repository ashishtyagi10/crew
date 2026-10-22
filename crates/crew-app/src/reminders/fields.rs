//! A todo and a reminder in the one shape the sync compares: the title,
//! whether it is done, and the due as Reminders keeps it, a day or a day and
//! a minute, rather than crew's instant. A due that went to Reminders and
//! came back is then the same due, not a change.
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::todopane::{duedate, item::TodoItem};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fields {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub done: bool,
    /// `2026-10-23`, or `2026-10-23T17:30` with a time of day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
}

const DAY: &str = "%Y-%m-%d";
const MINUTE: &str = "%Y-%m-%dT%H:%M";

impl Fields {
    pub(crate) fn of(it: &TodoItem) -> Fields {
        let due = it.due_ms.and_then(duedate::from_epoch_ms);
        Fields {
            title: it.title.clone(),
            done: it.done,
            due: due.map(|t| said(t.date(), it.due_has_time.then(|| t.time()))),
        }
    }

    /// The due as crew keeps it: an instant, and whether it has a time of
    /// day. A day alone sits at crew's default hour, as a typed `fri` does.
    pub(crate) fn due_ms(&self) -> Option<(u64, bool)> {
        let (day, at) = parts(self.due.as_deref()?)?;
        let time = at.or_else(|| NaiveTime::from_hms_opt(duedate::DEFAULT_HOUR, 0, 0))?;
        Some((duedate::to_epoch_ms(day.and_time(time))?, at.is_some()))
    }

    /// Whether two titles name the same thing: what pairs a todo with a
    /// reminder when nothing else does.
    pub(crate) fn same_title(&self, other: &Fields) -> bool {
        let t = |f: &Fields| f.title.trim().to_lowercase();
        !self.title.trim().is_empty() && t(self) == t(other)
    }
}

/// A day, and a time of day when there is one, as [`Fields::due`] says it.
pub(crate) fn said(day: NaiveDate, at: Option<NaiveTime>) -> String {
    match at {
        Some(t) => NaiveDateTime::new(day, t).format(MINUTE).to_string(),
        None => day.format(DAY).to_string(),
    }
}

/// [`said`] read back.
pub(crate) fn parts(due: &str) -> Option<(NaiveDate, Option<NaiveTime>)> {
    match NaiveDateTime::parse_from_str(due, MINUTE) {
        Ok(t) => Some((t.date(), Some(t.time()))),
        Err(_) => Some((NaiveDate::parse_from_str(due, DAY).ok()?, None)),
    }
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod tests;
