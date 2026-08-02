//! The grouped provider picker behind bare `/model` and `/model <n>`:
//! "your subscriptions" / "your keys" / "installed CLIs", numbered
//! continuously; signed-out delegated providers grayed with the exact
//! sign-in command. Split from `modelcmd.rs` to keep both inside the line
//! cap — this file owns the pure listing/selection, that one the construct.

use super::auth::state::{AuthState, ProviderInfo};

/// The numbered entries, in listing order: signed-in subscriptions, then
/// keys, then installed CLIs. `groups_text` numbers exactly this list, so
/// the two can never disagree about what `/model <n>` means.
pub(crate) fn selectable(states: &[ProviderInfo]) -> Vec<&ProviderInfo> {
    [
        AuthState::SignedIn,
        AuthState::KeyPresent,
        AuthState::Installed,
    ]
    .iter()
    .flat_map(|want| states.iter().filter(move |p| p.state == *want))
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
    let subs: Vec<String> = states
        .iter()
        .filter(|p| p.state == AuthState::SignedIn)
        .map(|p| numbered(p, "signed in"))
        .chain(
            states
                .iter()
                .filter(|p| p.state == AuthState::SignedOut)
                .map(|p| {
                    let login = p.login.unwrap_or_default();
                    format!(
                        " \u{25cb} {} \u{2014} signed out \u{00b7} sign in: run `{login}`",
                        p.name
                    )
                }),
        )
        .collect();
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
/// store (the same pin every model choice writes), so it survives restarts.
pub(crate) fn select(states: &[ProviderInfo], n: usize) -> String {
    let picks = selectable(states);
    let Some(p) = n.checked_sub(1).and_then(|i| picks.get(i)) else {
        return format!(
            "no provider #{n} \u{2014} the listing numbers 1..={}",
            picks.len()
        );
    };
    // An own-auth CLI (opencode) is an agent, not a model provider: there is
    // nothing to pin, and saying so beats silently doing nothing.
    if super::auth::registry::by_name(&p.name).is_none() {
        return format!(
            "{} carries its own sign-in \u{2014} address it with @{}",
            p.name, p.name
        );
    }
    match crate::credentials::save_pin(&p.name) {
        Ok(()) => format!(
            "provider pinned: {} \u{2014} smith work now routes there (persists across restarts)",
            p.name
        ),
        Err(e) => format!("could not store the {} pin: {e}", p.name),
    }
}

#[cfg(test)]
#[path = "modelpick_tests.rs"]
mod tests;
