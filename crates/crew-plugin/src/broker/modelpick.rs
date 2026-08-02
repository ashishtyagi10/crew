//! The grouped provider picker behind bare `/model` and `/model <n>`:
//! "your subscriptions" / "your keys" / "installed CLIs", numbered
//! continuously; signed-out delegated providers grayed with the exact
//! sign-in command. Split from `modelcmd.rs` to keep both inside the line
//! cap — this file owns the pure listing/selection, that one the construct.

use super::auth::state::{AuthState, ProviderInfo};

/// What picking a number does.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Pick {
    /// A message for the pane (pinned, out of range, own-auth CLI…).
    Note(String),
    /// Start this provider's device sign-in flow — the caller runs it on a
    /// worker (the poll can wait minutes; the stdio loop never does).
    SignIn(String),
}

/// The numbered entries, in listing order: signed-in subscriptions, then
/// signed-out device-flow providers (picking one signs in), then keys, then
/// installed CLIs. `groups_text` numbers exactly this list, so the two can
/// never disagree about what `/model <n>` means.
pub(crate) fn selectable(states: &[ProviderInfo]) -> Vec<&ProviderInfo> {
    states
        .iter()
        .filter(|p| p.state == AuthState::SignedIn)
        .chain(
            states
                .iter()
                .filter(|p| p.state == AuthState::SignedOut && p.device),
        )
        .chain(states.iter().filter(|p| p.state == AuthState::KeyPresent))
        .chain(states.iter().filter(|p| p.state == AuthState::Installed))
        .collect()
}

/// The grouped provider listing. Each group renders only when non-empty;
/// signed-out delegated providers ride under "your subscriptions" grayed
/// (`\u{25cb}`, unnumbered) with the exact sign-in command. Keyless and
/// uninstalled providers don't render at all — `/doctor` explains absences.
pub(crate) fn groups_text(states: &[ProviderInfo]) -> String {
    let mark = |p: &ProviderInfo| if p.active { " \u{2014} active" } else { "" };
    let mut n = 0usize;
    let mut numbered = |p: &ProviderInfo, detail: &str| {
        n += 1;
        format!(" {n}. {} \u{2014} {detail}{}", p.name, mark(p))
    };
    let mut groups: Vec<String> = Vec::new();
    let mut subs: Vec<String> = Vec::new();
    for p in states.iter().filter(|p| p.state == AuthState::SignedIn) {
        subs.push(numbered(p, "signed in"));
    }
    // Device-flow sign-ins are NUMBERED: picking one runs the flow right
    // here in the pane (code card, poll, done).
    for p in states
        .iter()
        .filter(|p| p.state == AuthState::SignedOut && p.device)
    {
        subs.push(numbered(
            p,
            "signed out \u{00b7} pick this number to sign in",
        ));
    }
    for p in states
        .iter()
        .filter(|p| p.state == AuthState::SignedOut && !p.device)
    {
        let login = p.login.unwrap_or_default();
        subs.push(format!(
            " \u{25cb} {} \u{2014} signed out \u{00b7} sign in: run `{login}`",
            p.name
        ));
    }
    if !subs.is_empty() {
        groups.push(format!("your subscriptions\n{}", subs.join("\n")));
    }
    let keys: Vec<String> = states
        .iter()
        .filter(|p| p.state == AuthState::KeyPresent)
        .map(|p| numbered(p, "key present"))
        .collect();
    if !keys.is_empty() {
        groups.push(format!("your keys\n{}", keys.join("\n")));
    }
    let clis: Vec<String> = states
        .iter()
        .filter(|p| p.state == AuthState::Installed)
        .map(|p| numbered(p, "carries its own sign-in"))
        .collect();
    if !clis.is_empty() {
        groups.push(format!("installed CLIs\n{}", clis.join("\n")));
    }
    if groups.is_empty() {
        return format!(
            "no providers yet \u{2014} {}",
            super::discover::no_provider_advice()
        );
    }
    format!(
        "providers \u{2014} /model <n> switches:\n{}",
        groups.join("\n")
    )
}

/// `/model <n>`: pin the n-th selectable provider through the credential
/// store (the same pin every model choice writes), so it survives restarts —
/// or, on a signed-out device-flow row, hand back the sign-in to run.
pub(crate) fn select(states: &[ProviderInfo], n: usize) -> Pick {
    let picks = selectable(states);
    let Some(p) = n.checked_sub(1).and_then(|i| picks.get(i)) else {
        return Pick::Note(format!(
            "no provider #{n} \u{2014} the listing numbers 1..={}",
            picks.len()
        ));
    };
    if p.state == AuthState::SignedOut && p.device {
        return Pick::SignIn(p.name.clone());
    }
    // An own-auth CLI (opencode) is an agent, not a model provider: there is
    // nothing to pin, and saying so beats silently doing nothing.
    if super::auth::registry::by_name(&p.name).is_none() {
        return Pick::Note(format!(
            "{} carries its own sign-in \u{2014} address it with @{}",
            p.name, p.name
        ));
    }
    Pick::Note(match crate::credentials::save_pin(&p.name) {
        Ok(()) => format!(
            "provider pinned: {} \u{2014} smith work now routes there (persists across restarts)",
            p.name
        ),
        Err(e) => format!("could not store the {} pin: {e}", p.name),
    })
}

#[cfg(test)]
#[path = "modelpick_tests.rs"]
mod tests;
