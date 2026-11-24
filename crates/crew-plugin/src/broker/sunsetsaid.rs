//! Smith's word that the model serving a reply has a shutdown date.
//!
//! WHY: a model pinned in config or `CREW_OPENAI_MODEL=gpt-5` keeps working
//! right up to its vendor's shutdown, then every turn fails, and nothing
//! before that day said so. Within `sunset::WARN_DAYS` of the date (or past
//! it), smith says `gpt-5 retires Dec 11 — gpt-6.1-sol next` under a reply —
//! ONCE a session (the broker process) per model: every reply carries the
//! same notice, so a line per turn would be noise.
use std::collections::HashSet;
use std::sync::Mutex;

use crew_hive::catalog::sunset::sunset;

/// The notice for a call that asked for `requested` (a model, a comma
/// chain, or empty for the provider default) and was answered by `served`:
/// the asked-for model first — after a fallback the retiring one is the
/// pin, not what answered — then the one that answered. Empty when neither
/// is near a date.
pub(crate) fn retiring(requested: &str, served: &str, today: i64) -> String {
    requested
        .split(',')
        .chain(std::iter::once(served))
        .map(str::trim)
        .find_map(|id| sunset(id, today).filter(|n| n.soon()).map(|n| n.line(id)))
        .unwrap_or_default()
}

/// An error from a call to `model` once its vendor's date has passed, with the reason the
/// error never gives: a retired model's calls fail, and no reply is left to carry the
/// notice under it (`retiring` rides only on a reply's usage). Unchanged otherwise, and for
/// a model the vendor re-routes rather than turns off.
pub(crate) fn why_failed(model: &str, err: String) -> String {
    why_failed_on(model, err, crew_hive::catalog::sunset::today())
}

/// [`why_failed`] as of `today` (days since 1970-01-01).
pub(crate) fn why_failed_on(model: &str, err: String, today: i64) -> String {
    match sunset(model, today).filter(|n| n.days_left <= 0 && !n.routed) {
        Some(n) => format!("{err} ({})", n.line(model)),
        None => err,
    }
}

/// [`why_failed`] for a hop's provider error, kept in its own variant's words.
pub(crate) fn why_failed_hop(
    model: &str,
    e: crew_hive::provider::ProviderError,
) -> crew_hive::provider::ProviderError {
    use crew_hive::provider::ProviderError as E;
    match e {
        E::Http(m) => E::Http(why_failed(model, m)),
        E::Api(m) => E::Api(why_failed(model, m)),
        E::Status(m) => E::Status(why_failed(model, m)),
        other => other,
    }
}

/// The notices already said; one set per process, which is the session.
#[derive(Default)]
pub(crate) struct Said(HashSet<String>);

impl Said {
    /// True the first time `line` comes by, false for empty and repeats.
    pub(crate) fn first(&mut self, line: &str) -> bool {
        !line.is_empty() && self.0.insert(line.to_string())
    }
}

static SAID: Mutex<Option<Said>> = Mutex::new(None);

/// [`Said::first`] on this session's set.
pub(crate) fn first_in_session(line: &str) -> bool {
    if line.is_empty() {
        return false;
    }
    let mut g = SAID.lock().unwrap_or_else(|p| p.into_inner());
    g.get_or_insert_with(Said::default).first(line)
}

#[cfg(test)]
#[path = "sunsetsaid_tests.rs"]
mod tests;
