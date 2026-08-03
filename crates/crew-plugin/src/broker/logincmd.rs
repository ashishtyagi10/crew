//! `/login` and `/logout` — the sign-in front door. `/login` lists every
//! provider that offers a sign-in (crew's native device flow, or a vendor
//! CLI's own login) and runs the device flow by name or number; `/logout`
//! removes a stored grant. Exists because the sign-in affordance lived only
//! inside `/model <n>` AND only rendered when no key was present — a user
//! holding a key in a shell rc had no visible OAuth path at all.
use crate::PluginEvent;

use super::auth::probe::{self, CliAuth};
use super::auth::registry;
use super::relay::msg;
use super::session::Session;

/// One provider that offers a sign-in — pure data, so the listing and the
/// pick are testable without a keychain or a CLI probe.
#[derive(Clone, Debug)]
pub(crate) struct LoginRow {
    pub name: String,
    /// `Some(command)` when a vendor CLI owns the sign-in (delegated rung).
    pub cli_login: Option<&'static str>,
    /// Whether crew's native device flow can sign this provider in.
    pub device: bool,
    pub signed_in: bool,
    /// An API key is present alongside (device rows only) — worth naming,
    /// because a key used to hide the sign-in affordance entirely.
    pub key_present: bool,
}

/// The live rows: device-flow providers always (signing in is valid with or
/// without a key), delegated CLIs when installed and probeable. An own-auth
/// CLI (probe `Unknown`) manages itself and has nothing to list here.
pub(crate) fn rows() -> Vec<LoginRow> {
    let store = crate::credentials::load();
    let mut out = Vec::new();
    for e in registry::entries() {
        if e.device.is_some() {
            out.push(LoginRow {
                name: e.name.to_string(),
                cli_login: None,
                device: true,
                signed_in: super::auth::tokens::load(e.name).is_some(),
                key_present: e
                    .key_var
                    .is_some_and(|v| super::discover::key_raw(&store, v).is_some()),
            });
        } else if let Some(cli) = e.cli {
            let signed_in = match probe::state_cached(&cli) {
                CliAuth::SignedIn => true,
                CliAuth::SignedOut => false,
                CliAuth::Absent | CliAuth::Unknown => continue,
            };
            out.push(LoginRow {
                name: e.name.to_string(),
                cli_login: Some(cli.login),
                device: false,
                signed_in,
                key_present: false,
            });
        }
    }
    out
}

/// The `/login` listing: device-flow rows numbered (picking one signs in,
/// key or no key), delegated CLIs grayed with their exact command. Pure.
pub(crate) fn listing(rows: &[LoginRow]) -> String {
    if rows.is_empty() {
        return format!(
            "no provider here offers a sign-in \u{2014} {}",
            super::discover::no_provider_advice()
        );
    }
    let mut lines = vec!["sign in \u{2014} /login <n|name> runs the flow right here:".to_string()];
    let mut n = 0usize;
    for r in rows.iter().filter(|r| r.device) {
        n += 1;
        let detail = match (r.signed_in, r.key_present) {
            (true, _) => "\u{2713} signed in \u{00b7} /logout removes the grant".to_string(),
            (false, true) => format!(
                "key present \u{00b7} /login {n} signs in with OAuth instead \
                 (the grant then serves)"
            ),
            (false, false) => format!("signed out \u{00b7} /login {n} signs in"),
        };
        lines.push(format!(" {n}. {} \u{2014} {detail}", r.name));
    }
    for r in rows.iter().filter(|r| !r.device) {
        let detail = if r.signed_in {
            "\u{2713} signed in (vendor CLI)".to_string()
        } else {
            format!(
                "signed out \u{00b7} run `{}`",
                r.cli_login.unwrap_or_default()
            )
        };
        lines.push(format!(" \u{25cb} {} \u{2014} {detail}", r.name));
    }
    lines.join("\n")
}

/// What `/login <arg>` resolves to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LoginPick {
    /// Run this provider's device flow (a signed-in pick re-authenticates).
    Device(String),
    /// A message for the pane.
    Note(String),
}

/// Resolve a number (into the numbered device rows, exactly as [`listing`]
/// prints them) or a provider name. Pure.
pub(crate) fn pick(rows: &[LoginRow], arg: &str) -> LoginPick {
    let device: Vec<&LoginRow> = rows.iter().filter(|r| r.device).collect();
    if let Ok(n) = arg.parse::<usize>() {
        return match n.checked_sub(1).and_then(|i| device.get(i)) {
            Some(r) => LoginPick::Device(r.name.clone()),
            None => LoginPick::Note(format!(
                "no sign-in #{n} \u{2014} the listing numbers 1..={}",
                device.len()
            )),
        };
    }
    let Some(r) = rows.iter().find(|r| r.name.eq_ignore_ascii_case(arg)) else {
        return LoginPick::Note(match registry::by_name(arg) {
            Some(e) => format!(
                "{} offers no sign-in flow \u{2014} paste a key via /model",
                e.name
            ),
            None => format!(
                "unknown provider \u{201c}{arg}\u{201d} \u{2014} /login lists who can sign in"
            ),
        });
    };
    match r.cli_login {
        Some(login) => LoginPick::Note(format!(
            "{} signs in through its own CLI \u{2014} run `{login}`; \
             crew picks it up automatically",
            r.name
        )),
        None => LoginPick::Device(r.name.clone()),
    }
}

/// `/login [provider|n]` — list, or run the device sign-in (the caller
/// routes an argument form as a background task; the poll can wait minutes).
pub(crate) fn login_cmd(
    session: &Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let arg = rest.trim();
    let rows = rows();
    if arg.is_empty() {
        return emit(msg("agent smith", listing(&rows)));
    }
    match pick(&rows, arg) {
        LoginPick::Device(name) => super::signin::signin_cmd(session, &name, emit),
        LoginPick::Note(note) => emit(msg("agent smith", note)),
    }
}

/// `/logout [provider]` — remove a stored OAuth grant. With a key still
/// present the provider serves from the key again; with neither it returns
/// to the sign-in affordance. Bare `/logout` resolves alone only when
/// exactly one grant exists — never guess which sign-in to destroy.
pub(crate) fn logout_cmd(
    session: &Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let arg = rest.trim();
    let signed: Vec<String> = rows()
        .iter()
        .filter(|r| r.device && r.signed_in)
        .map(|r| r.name.clone())
        .collect();
    let name = match (arg.is_empty(), signed.len()) {
        (true, 1) => signed[0].clone(),
        (true, 0) => return emit(msg("agent smith", "no stored sign-in to remove")),
        (true, _) => {
            return emit(msg(
                "agent smith",
                format!(
                    "signed in: {} \u{2014} /logout <name> picks one",
                    signed.join(", ")
                ),
            ))
        }
        (false, _) => arg.to_string(),
    };
    if !signed.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
        return emit(msg("agent smith", format!("no stored sign-in for {name}")));
    }
    super::auth::tokens::clear(&name);
    emit(PluginEvent::Roster {
        agents: session.registry().infos(),
    })?;
    emit(msg(
        "agent smith",
        format!(
            "signed out \u{2014} {name}'s grant removed \
             (an API key, if present, serves again)"
        ),
    ))
}

#[cfg(test)]
#[path = "logincmd_tests.rs"]
mod tests;
