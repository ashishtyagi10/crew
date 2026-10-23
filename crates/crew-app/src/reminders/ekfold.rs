//! The `crew` list of v0.28.4–97 folded into the list new reminders go to,
//! once: the sync now keeps every list, as "All" on the phone shows them (the
//! user, 2026-10-23: "sync with reminder should happen to 'all' instead of
//! crew section"), and a list of crew's own was a section of its own there.
use objc2_event_kit::EKCalendar;
use objc2_foundation::{NSArray, NSString};

use super::ek::{err, Kit};

/// Reminders moved, by id before and after, and whether the list is gone.
pub(super) type Moved = (Vec<(String, String)>, bool);

impl Kit {
    /// The `crew` list v0.28.4–97 kept, `id`, folded into `home`: each of its
    /// reminders moved there, then the list deleted, once nothing is left in
    /// it (deleting a list deletes what it holds). Each moved reminder's id
    /// before and after, and whether the list is gone.
    pub(super) fn fold(&self, id: &str, home: &EKCalendar) -> Result<Moved, String> {
        let old = unsafe { self.store.calendarWithIdentifier(&NSString::from_str(id)) };
        let Some(old) = old.filter(|o| Kit::id(o) != Kit::id(home)) else {
            return Ok((Vec::new(), true));
        };
        let list = NSArray::from_retained_slice(std::slice::from_ref(&old));
        let staged = (|| {
            let mut went = Vec::new();
            for (was, _) in self.fetch(&list)? {
                let r =
                    (self.find(&was)).ok_or("Reminders: a reminder went while crew moved it")?;
                unsafe { r.setCalendar(Some(home)) };
                unsafe { self.store.saveReminder_commit_error(&r, false) }.map_err(err)?;
                went.push((was, r));
            }
            self.commit().map(|_| went)
        })();
        let went = staged.inspect_err(|_| self.rollback())?;
        let moved = (went.into_iter())
            .map(|(was, r)| (was, unsafe { r.calendarItemIdentifier() }.to_string()))
            .collect();
        if !self.fetch(&list)?.is_empty() {
            return Ok((moved, false));
        }
        unsafe { self.store.removeCalendar_commit_error(&old, true) }.map_err(err)?;
        Ok((moved, true))
    }
}
