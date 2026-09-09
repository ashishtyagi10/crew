//! "The last choice serves": a sign-in that happens in a terminal is a
//! choice too, and it must outrank a pin written earlier. Crew cannot see
//! WHEN a vendor CLI was signed in, but it can see the verdict CHANGE — the
//! credential store keeps each CLI's last definitive verdict
//! (`Store::seen`), and a flip is the moment the choice was made: out→in
//! pins that provider, in→out drops a pin that named it, so discovery
//! decides again. First sightings only record: an install that was signed
//! in all along is not a new choice, and must not unseat a pin the user
//! wrote after it.
use super::probe::{self, CliAuth};
use super::registry;

/// What a fresh verdict means against the recorded one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Change {
    /// Signed out (or first seen) before, signed in now.
    SignedIn,
    /// Signed in before, signed out now.
    SignedOut,
    /// Nothing to act on.
    None,
}

/// Pure: the verdict to record (`None` = not definitive, record nothing)
/// and the change it means against `last` (`"in"`/`"out"`/absent).
pub(crate) fn change(last: Option<&str>, now: CliAuth) -> (Option<&'static str>, Change) {
    match now {
        CliAuth::SignedIn => (
            Some("in"),
            if last == Some("out") {
                Change::SignedIn
            } else {
                Change::None
            },
        ),
        CliAuth::SignedOut => (
            Some("out"),
            if last == Some("in") {
                Change::SignedOut
            } else {
                Change::None
            },
        ),
        // Not installed, or the CLI could not answer: no verdict, and no
        // change — a probe timeout must never look like a sign-out.
        CliAuth::Absent | CliAuth::Unknown => (None, Change::None),
    }
}

/// Pure: the store edits a change calls for, as `(record, pin)` where
/// `pin` is `Some(Some(name))` to pin, `Some(None)` to clear, `None` to
/// leave the pin alone. A sign-out clears only a pin naming THIS provider.
pub(crate) fn edits(
    name: &str,
    pinned: Option<&str>,
    last: Option<&str>,
    now: CliAuth,
) -> (Option<&'static str>, Option<Option<String>>) {
    let (record, ch) = change(last, now);
    let record = record.filter(|r| Some(*r) != last);
    let pin = match ch {
        Change::SignedIn => Some(Some(name.to_string())),
        Change::SignedOut if pinned == Some(name) => Some(None),
        Change::SignedOut | Change::None => None,
    };
    (record, pin)
}

/// Apply [`edits`] for every CLI-owned sign-in the registry knows, against
/// the live (cached) probes. Returns whether the store changed, so the
/// caller re-reads it before resolving. Best-effort: a store that cannot
/// be written changes nothing, as everywhere else.
pub(crate) fn observe(store: &crate::credentials::Store) -> bool {
    let mut changed = false;
    for e in registry::entries() {
        let Some(cli) = e.cli.or(e.mint.map(|m| m.cli)) else {
            continue;
        };
        let now = probe::state_cached(&cli);
        let last = store.seen.get(e.name).map(String::as_str);
        let (record, pin) = edits(e.name, store.provider.as_deref(), last, now);
        if let Some(r) = record {
            changed |= crate::credentials::save_seen(e.name, r).is_ok();
        }
        match pin {
            Some(Some(name)) => changed |= crate::credentials::save_pin(&name).is_ok(),
            Some(None) => changed |= crate::credentials::clear_pin().is_ok(),
            None => {}
        }
    }
    changed
}

#[cfg(test)]
#[path = "seen_tests.rs"]
mod tests;
