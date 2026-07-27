//! Persisting a provider key submitted through [`crate::keyentry`]. Split out
//! of `chat.rs` (already well over its own budget) rather than grown there,
//! matching how `chattheme`/`chatexport` keep their own composer intercepts
//! as free functions over `&mut ChatPane`.
use std::path::Path;

use crate::chat::ChatPane;

/// Persist a provider key supplied in-app, pin its provider, and make the
/// picker resolve against it immediately. Reports what happened by NAME —
/// the value never reaches a message, a log or the transcript.
pub(crate) fn store_provider_key(pane: &mut ChatPane, var: &str, value: &str) {
    match crew_plugin::credentials::path() {
        Some(path) => store_provider_key_at(pane, &path, var, value),
        None => pane.push_note(format!(
            "could not save {var}: no config directory to store credentials in"
        )),
    }
}

/// [`store_provider_key`] against an explicit destination path — the
/// testable half. `crew_plugin::credentials` deliberately exposes
/// `save_key_at` as its own test seam (see its module doc) rather than
/// resolving the real path internally, precisely so a caller like this one
/// can be exercised without writing to the user's real credentials file or
/// mutating the process-global `CREW_CREDENTIALS_PATH`. Never logs `value`.
pub(crate) fn store_provider_key_at(pane: &mut ChatPane, path: &Path, var: &str, value: &str) {
    let provider = crew_plugin::credentials::provider_for(var);
    let line = match crew_plugin::credentials::save_key_at(path, var, value, provider) {
        Ok(()) => {
            crate::shellprobe::note_key(var, value);
            match provider {
                Some(p) => {
                    // The pin the store just recorded has to reach the app's
                    // own resolution too, or the row stays dim, accepting it
                    // re-opens this same prompt, and the app and the broker
                    // disagree about which provider is active.
                    crate::shellprobe::note_pin(p);
                    format!("{var} saved · {p} pinned")
                }
                None => format!("{var} saved"),
            }
        }
        Err(e) => format!("could not save {var}: {e}"),
    };
    pane.push_note(line);
}

#[cfg(test)]
#[path = "chatkeystore_tests.rs"]
mod tests;
