//! `/login` and `/logout` — the sign-in front door. `/login` lists every
//! provider that offers a sign-in (crew's native device flow, or a vendor
//! CLI's own login) and runs the device flow by name or number; `/logout`
//! removes a stored grant. Exists because the sign-in affordance lived only
//! inside `/model <n>` AND only rendered when no key was present — a user
//! holding a key in a shell rc had no visible OAuth path at all. The pure
//! rows/listing/pick live in `loginrows`; this file gathers and runs.
use crate::PluginEvent;

pub(crate) use super::loginrows::{listing, pick, LoginPick, LoginRow};

use super::auth::probe::{self, CliAuth};
use super::auth::registry;
use super::relay::msg;
use super::session::Session;

/// The live rows: device-flow providers always (signing in is valid with or
/// without a key), delegated CLIs when installed and probeable, minting
/// CLIs even when absent (the row says how to install — the whole point of
/// the front door is that the path is visible). An own-auth CLI (probe
/// `Unknown`) manages itself and has nothing to list here.
pub(crate) fn rows() -> Vec<LoginRow> {
    let store = crate::credentials::load();
    let mut out = Vec::new();
    for e in registry::entries() {
        let key_present = e
            .key_var
            .is_some_and(|v| super::discover::key_raw(&store, v).is_some());
        if e.device.is_some() {
            out.push(LoginRow {
                name: e.name.to_string(),
                cli_login: None,
                device: true,
                signed_in: super::auth::tokens::load(e.name).is_some(),
                key_present,
                install: None,
            });
        } else if let Some(m) = e.mint {
            let (signed_in, install) = match probe::state_cached(&m.cli) {
                CliAuth::SignedIn => (true, None),
                CliAuth::SignedOut => (false, None),
                CliAuth::Absent => (false, Some(m.install)),
                CliAuth::Unknown => continue,
            };
            out.push(LoginRow {
                name: e.name.to_string(),
                cli_login: Some(m.cli.login),
                device: false,
                signed_in,
                key_present,
                install,
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
                install: None,
            });
        }
    }
    out
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
        // A minting CLI owns its store: crew deletes nothing there.
        let note = match registry::by_name(&name).and_then(|e| e.mint) {
            Some(m) => format!(
                "{name} signs out through its own CLI \u{2014} run `{}`",
                m.logout
            ),
            None => format!("no stored sign-in for {name}"),
        };
        return emit(msg("agent smith", note));
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
