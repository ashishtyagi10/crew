//! macOS's question, asked once: may crew see your reminders. Its answer,
//! and the words the sync says when it is no.
use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::sel;
use objc2_event_kit::{EKEntityType, EKEventStore};
use objc2_foundation::NSObjectProtocol;
use objc2_foundation::{NSBundle, NSError, NSString};

pub(super) const REFUSED: &str = "Reminders: crew may not see your reminders \u{b7} System Settings \u{203a} Privacy & Security \u{203a} Reminders, then /todo reminders";
/// macOS ends an app that asks without saying why, so the ask is not made
/// without the note (`regmac`'s Info.plist and the one built into crew).
const NO_NOTE: &str = "Reminders: this crew cannot ask for Reminders \u{b7} open it from Crew.app";
const USAGE_KEY: &str = "NSRemindersFullAccessUsageDescription";

/// macOS's question, and the answer: blocks until it is given.
pub(super) fn ask() -> Result<(), String> {
    let key = NSString::from_str(USAGE_KEY);
    if NSBundle::mainBundle()
        .objectForInfoDictionaryKey(&key)
        .is_none()
    {
        return Err(NO_NOTE.into());
    }
    let (tx, rx) = mpsc::channel();
    let block = RcBlock::new(move |yes: Bool, _: *mut NSError| {
        let _ = tx.send(yes.as_bool());
    });
    let store = unsafe { EKEventStore::new() };
    // Full access is macOS 14's word for it; before, one ask per kind.
    let full = sel!(requestFullAccessToRemindersWithCompletion:);
    match store.respondsToSelector(full) {
        true => unsafe {
            store.requestFullAccessToRemindersWithCompletion(RcBlock::as_ptr(&block))
        },
        #[allow(deprecated)]
        false => unsafe {
            store.requestAccessToEntityType_completion(
                EKEntityType::Reminder,
                RcBlock::as_ptr(&block),
            )
        },
    }
    match rx.recv_timeout(Duration::from_secs(600)) {
        Ok(true) => Ok(()),
        _ => Err(REFUSED.into()),
    }
}
