//! The sign-in rows of the `/model` picker — one front door for "who
//! serves": sign in here, then pick a model the sign-in serves, in the
//! same popup. The rows are the broker's `SignInOption`s, which ride every
//! `Roster` event (`chatdrain`) so the picker always shows the state the
//! broker last saw; `serving` is the provider that event named. A pick
//! submits `/login <name>` — the broker signs in (a device flow), or makes
//! an already signed-in CLI serve (the pin moves: the last choice wins).
use std::sync::RwLock;

use crew_plugin::SignInOption;

use crate::suggest::MenuItem;

static SIGNINS: RwLock<Vec<SignInOption>> = RwLock::new(Vec::new());
static SERVING: RwLock<Option<String>> = RwLock::new(None);

/// Record the broker's latest rows and serving provider.
pub(crate) fn set(options: Vec<SignInOption>, provider: Option<String>) {
    if let Ok(mut s) = SIGNINS.write() {
        *s = options;
    }
    if let Ok(mut p) = SERVING.write() {
        *p = provider;
    }
}

pub(crate) fn now() -> Vec<SignInOption> {
    SIGNINS.read().map(|s| s.clone()).unwrap_or_default()
}

pub(crate) fn serving() -> Option<String> {
    SERVING.read().ok().and_then(|p| p.clone())
}

/// Whether `options` say `name` is signed in (device grant or vendor CLI).
pub(crate) fn signed_in(options: &[SignInOption], name: &str) -> bool {
    options
        .iter()
        .any(|o| o.signed_in && o.name.eq_ignore_ascii_case(name))
}

/// What a row says after its name, given who serves right now.
fn desc(o: &SignInOption, serving: bool) -> String {
    match (o.signed_in, serving) {
        (true, true) => "\u{2713} signed in \u{00b7} serving".into(),
        (true, false) => "\u{2713} signed in \u{00b7} pick to make it serve".into(),
        (false, _) => crate::loginpick::state(o),
    }
}

/// [`section`] over the live serving provider, with OpenRouter's key state
/// read off the keys the app holds (`held`).
pub(crate) fn section_now(
    options: &[SignInOption],
    held: &std::collections::HashSet<String>,
    query: &str,
) -> Vec<MenuItem> {
    let keyed = held.contains(crate::oauth::OPENROUTER_KEY_VAR);
    section(options, keyed, serving().as_deref(), query)
}

/// The section: a header, then every sign-in matching `query` — device
/// flows and vendor CLIs from `options`, plus OpenRouter's browser flow
/// (the app's own, so it is never in `options`). A signed-out CLI row is
/// dim: the pick tells the user the command to run. Empty `options` means
/// no broker has spoken yet, and the section stays out entirely.
pub(crate) fn section(
    options: &[SignInOption],
    openrouter_keyed: bool,
    serving: Option<&str>,
    query: &str,
) -> Vec<MenuItem> {
    if options.is_empty() {
        return Vec::new();
    }
    let hit = |name: &str| query.is_empty() || name.to_lowercase().contains(query);
    let mut out = vec![MenuItem {
        label: "sign in \u{00b7} who serves".into(),
        header: true,
        ..Default::default()
    }];
    for o in options.iter().filter(|o| hit(&o.name)) {
        let serves = serving.is_some_and(|p| p.eq_ignore_ascii_case(&o.name));
        out.push(MenuItem {
            label: o.name.clone(),
            desc: desc(o, serves),
            fill: format!("/login {}", o.name),
            submit: true,
            dim: !o.signed_in && !o.device,
            ..Default::default()
        });
    }
    if hit("openrouter") {
        out.push(MenuItem {
            label: "openrouter".into(),
            desc: if openrouter_keyed {
                "\u{2713} key present \u{00b7} pick to sign in again in the browser".into()
            } else {
                "sign in in the browser".into()
            },
            fill: "openrouter".into(),
            needs: Some(crate::oauth::OPENROUTER_KEY_VAR.to_string()),
            ..Default::default()
        });
    }
    if out.len() == 1 {
        return Vec::new(); // never an empty section
    }
    out
}

#[cfg(test)]
#[path = "modelsignin_tests.rs"]
mod tests;
