//! A failure in the words the foot says: what it is, where crew can name it
//! (`states::known`), else the service's own words. A grant made for the
//! read-only scope (v0.27.29) cannot archive, star or send, and Google adds
//! no scope to an installed app's grant without a fresh consent.
//!
//! An invite's answer or add fails as `not answered: …` / `not added: …`,
//! and the head stays: worded whole, "not answered: could not reach Gmail"
//! came out as "Gmail can't be reached from here", and nothing said the
//! answer had not gone. Nor is Google Calendar called Gmail: its 409 read
//! "Gmail 409", its slow-down "Gmail asked crew to slow down".

pub(crate) fn told(e: String) -> String {
    if let Some((head, rest)) = e.split_once(": ").filter(|(h, _)| h.starts_with("not ")) {
        return format!("{head}: {}", told(rest.to_string()));
    }
    if e.contains("insufficient authentication scopes") {
        return "archiving, starring and sending need a new sign-in: /inbox signout, then Enter"
            .into();
    }
    let calendar = e.to_ascii_lowercase().contains("google calendar");
    match super::super::states::known(&e) {
        Some((what, _)) if calendar => what.replace("Gmail", "Google Calendar"),
        Some((what, _)) => what.to_string(),
        None => e,
    }
}

#[cfg(test)]
#[path = "told_tests.rs"]
mod tests;
