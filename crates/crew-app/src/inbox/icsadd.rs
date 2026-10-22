//! `c` in the reader: the open thread's invite, added to your calendar by
//! the system's calendar app (goal 2026-10-22-inbox-calendar, phase 2). crew
//! writes the invite as it came to a file of its own and opens that: Calendar
//! on a Mac, whichever app opens `.ics` files elsewhere. That app shows its
//! own page to add or answer it and writes to whichever account you keep in
//! it (iCloud, Google, Exchange). Nothing goes to Google from crew.
use super::super::InboxPane;
use super::icstz::{place, Placed};

impl InboxPane {
    /// Add the open thread's newest invite: to Google Calendar once it has
    /// answered for it (`icsanswer`), else here. What happened, for the note.
    pub(crate) fn add_to_calendar(&mut self) -> String {
        let Some(ics) = self.invite() else {
            return "no invite in this thread".into();
        };
        let Some(inv) = super::parse(&ics) else {
            return "crew cannot read this invite".into();
        };
        if inv.cancelled {
            return "this meeting was cancelled \u{b7} nothing to add".into();
        }
        // A wrong time is worse than none (the goal's third rule).
        if let Some(Placed::Unplaced(_, zone)) = inv.start.as_ref().map(|s| place(s, &inv.zones)) {
            return format!("crew cannot place the zone {zone}, so it adds nothing");
        }
        match self.on_google(&inv.uid) {
            Some(Some(_)) => return "already on your Google Calendar".into(),
            Some(None) => {
                self.ask(crate::inbox::worker::Ask::AddEvent(ics));
                return "adding it to your Google Calendar\u{2026}".into();
            }
            None => {}
        }
        let path = std::env::temp_dir().join(file_name(&inv.uid));
        if let Err(e) = std::fs::write(&path, &ics) {
            return format!("could not write the invite: {e}");
        }
        crate::linkclick::open_external(&path.to_string_lossy());
        match cfg!(target_os = "macos") {
            true => "opened in Calendar \u{b7} add it there".into(),
            false => "opened in your calendar app \u{b7} add it there".into(),
        }
    }
}

/// `crew-invite-<uid>.ics`, the uid cut to characters safe in a file name.
pub(crate) fn file_name(uid: &str) -> String {
    let safe: String = uid
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || "-_".contains(*c))
        .take(48)
        .collect();
    match safe.is_empty() {
        true => "crew-invite.ics".into(),
        false => format!("crew-invite-{safe}.ics"),
    }
}

#[cfg(test)]
#[path = "icsadd_tests.rs"]
mod tests;
