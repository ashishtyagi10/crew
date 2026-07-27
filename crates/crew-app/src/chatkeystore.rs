//! Persisting a provider key submitted through [`crate::keyentry`]. Split out
//! of `chat.rs` (already well over its own budget) rather than grown there,
//! matching how `chattheme`/`chatexport` keep their own composer intercepts
//! as free functions over `&mut ChatPane`.
use crate::chat::ChatPane;

/// Persist a provider key supplied in-app, pin its provider, and make the
/// picker resolve against it immediately. Reports what happened by NAME —
/// the value never reaches a message, a log or the transcript.
pub(crate) fn store_provider_key(pane: &mut ChatPane, var: &str, value: &str) {
    let provider = crew_plugin::credentials::provider_for(var);
    let line = match crew_plugin::credentials::save_key(var, value, provider) {
        Ok(()) => {
            crate::shellprobe::note_key(var, value);
            match provider {
                Some(p) => format!("{var} saved · {p} pinned"),
                None => format!("{var} saved"),
            }
        }
        Err(e) => format!("could not save {var}: {e}"),
    };
    pane.push_note(line);
}
