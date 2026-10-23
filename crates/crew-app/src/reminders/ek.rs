//! EventKit, as little of it as the sync needs: access, the `crew` list, its
//! reminders read as [`Fields`], and writes staged until one commit. Every
//! call runs on the sync's own thread.
use std::ptr::NonNull;
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2_event_kit::{
    EKAuthorizationStatus, EKCalendar, EKEntityType, EKEventStore, EKEventStoreChangedNotification,
    EKReminder,
};
use objc2_foundation::{
    NSArray, NSError, NSNotification, NSNotificationCenter, NSObjectProtocol, NSString,
};

use super::ekask::{ask, REFUSED};
use super::ekfields::{components, read, ring};
use super::fields::Fields;
use super::pairs::{self, Pick};

/// The list's name in Reminders.
const LIST: &str = "crew";

pub(super) type Watch = Retained<ProtocolObject<dyn NSObjectProtocol>>;

pub(super) struct Kit {
    store: Retained<EKEventStore>,
}

fn err(e: Retained<NSError>) -> String {
    format!("Reminders: {}", e.localizedDescription())
}

impl Kit {
    /// The store, once crew may use it: asked here the first time.
    pub(super) fn open() -> Result<Kit, String> {
        let status =
            unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Reminder) };
        if status == EKAuthorizationStatus::NotDetermined {
            ask()?;
        } else if status != EKAuthorizationStatus::FullAccess {
            return Err(REFUSED.into());
        }
        // A store made before the answer would not see the lists.
        Ok(Kit {
            store: unsafe { EKEventStore::new() },
        })
    }

    /// The `crew` list ([`pairs::pick`]): the one remembered by `id`, else
    /// the one so named, else a new one in the account new reminders go to.
    /// True when it is not the one remembered.
    pub(super) fn list(
        &self,
        id: &str,
        remake: bool,
    ) -> Result<(Retained<EKCalendar>, bool), String> {
        unsafe {
            let known = (!id.is_empty())
                .then(|| self.store.calendarWithIdentifier(&NSString::from_str(id)))
                .flatten();
            let all = self.store.calendarsForEntityType(EKEntityType::Reminder);
            let named = all.iter().find(|c| c.title().to_string() == LIST);
            match pairs::pick(id, known.is_some(), named.is_some(), remake) {
                Pick::Deleted => return Err(pairs::DELETED.into()),
                Pick::Make => {}
                p => return Ok((known.or(named).ok_or(pairs::DELETED)?, p == Pick::Named)),
            }
            let c =
                EKCalendar::calendarForEntityType_eventStore(EKEntityType::Reminder, &self.store);
            c.setTitle(&NSString::from_str(LIST));
            let home = self.store.defaultCalendarForNewReminders();
            let source = home
                .and_then(|d| d.source())
                .ok_or("Reminders: no account to keep a crew list in")?;
            c.setSource(Some(&source));
            self.store
                .saveCalendar_commit_error(&c, true)
                .map_err(err)?;
            Ok((c, true))
        }
    }

    pub(super) fn id(list: &EKCalendar) -> String {
        unsafe { list.calendarIdentifier() }.to_string()
    }

    /// Every reminder in `list`, done ones too, by id.
    pub(super) fn fetch(&self, list: &EKCalendar) -> Result<Vec<(String, Fields)>, String> {
        let (tx, rx) = mpsc::channel();
        let block = RcBlock::new(move |found: *mut NSArray<EKReminder>| {
            let rows = unsafe { found.as_ref() }.map(|a| a.iter().map(|r| read(&r)).collect());
            let _ = tx.send(rows);
        });
        unsafe {
            let which = self
                .store
                .predicateForRemindersInCalendars(Some(&NSArray::from_slice(&[list])));
            self.store
                .fetchRemindersMatchingPredicate_completion(&which, &block);
        }
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(Some(rows)) => Ok(rows),
            Ok(None) => Err("Reminders: the crew list could not be read".into()),
            Err(_) => Err("Reminders did not answer in a minute".into()),
        }
    }

    /// `f` into reminder `id`, saying `was` now, or into a new one in
    /// `list`: staged until [`Kit::commit`]. The reminder's id.
    pub(super) fn put(
        &self,
        list: &EKCalendar,
        id: Option<&str>,
        was: Option<&Fields>,
        f: &Fields,
    ) -> Result<String, String> {
        let r = match id {
            Some(id) => self
                .find(id)
                .ok_or("Reminders: a reminder went while crew wrote it")?,
            None => unsafe {
                let r = EKReminder::reminderWithEventStore(&self.store);
                r.setCalendar(Some(list));
                r
            },
        };
        unsafe {
            r.setTitle(Some(&NSString::from_str(&f.title)));
            if r.isCompleted() != f.done {
                r.setCompleted(f.done);
            }
            if was.is_none_or(|w| w.due != f.due) {
                r.setDueDateComponents(f.due.as_deref().and_then(components).as_deref());
                ring(&r, f);
            }
            self.store
                .saveReminder_commit_error(&r, false)
                .map_err(err)?;
            Ok(r.calendarItemIdentifier().to_string())
        }
    }

    /// Reminder `id` out of the list, staged; gone already is fine.
    pub(super) fn remove(&self, id: &str) -> Result<(), String> {
        match self.find(id) {
            Some(r) => unsafe { self.store.removeReminder_commit_error(&r, false) }.map_err(err),
            None => Ok(()),
        }
    }

    pub(super) fn commit(&self) -> Result<(), String> {
        unsafe { self.store.commit() }.map_err(err)
    }

    /// Everything staged since the last commit, let go.
    pub(super) fn rollback(&self) {
        unsafe { self.store.reset() }
    }

    fn find(&self, id: &str) -> Option<Retained<EKReminder>> {
        let item = unsafe {
            self.store
                .calendarItemWithIdentifier(&NSString::from_str(id))
        }?;
        item.downcast::<EKReminder>().ok()
    }

    /// `tx` told whenever Reminders changes, here or on another device.
    pub(super) fn watch(&self, tx: mpsc::Sender<()>) -> Watch {
        let block = RcBlock::new(move |_: NonNull<NSNotification>| {
            let _ = tx.send(());
        });
        let store: &AnyObject = &self.store;
        unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(EKEventStoreChangedNotification),
                Some(store),
                None,
                &block,
            )
        }
    }

    pub(super) fn unwatch(w: &Watch) {
        let w: &AnyObject = w.as_ref();
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(w) }
    }
}
