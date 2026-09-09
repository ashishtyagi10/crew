//! `/login` and `/logout` — the sign-in front door. `/login` lists every
//! provider that offers a sign-in (crew's native device flow, or a vendor
//! CLI's own login) and runs the device flow by name or number; `/logout`
//! removes a stored grant. Exists because the sign-in affordance lived only
//! inside `/model <n>` AND only rendered when no key was present — a user
//! holding a key in a shell rc had no visible OAuth path at all. The pure
//! rows/listing/pick live in `loginrows`; this file gathers and runs.
use crate::PluginEvent;

pub(crate) use super::loginrows::{listing, options, pick, signed_in, LoginPick, LoginRow};

use super::auth::probe::{self, CliAuth};
use super::auth::registry;
use super::relay::msg;
use super::session::Session;

/// The live rows: device-flow providers always (signing in is valid with or
/// without a key), delegated CLIs when installed and probeable, minting
/// CLIs even when absent (the row says how to install — the whole point of
/// the front door is that the path is visible). An own-auth CLI (probe
/// `Unknown`) manages itself and has nothing to list here.
///
/// The CLIs are probed FRESH here, not from the cache: `/login` is the
/// moment a user checks whether the `ant auth login` they just ran in a
/// terminal took, and a cached "not installed" from the pane's first
/// minute answered "no" to a machine that had it.
pub(crate) fn rows() -> Vec<LoginRow> {
    rows_with(&probe::state_fresh)
}

/// [`rows`] from the per-process probe cache — for the roster event, which
/// goes out after every provider change and must not spawn three CLIs
/// each time. A negative verdict re-probes within `probe::NEGATIVE_TTL`.
pub(crate) fn rows_cached() -> Vec<LoginRow> {
    rows_with(&probe::state_cached)
}

fn rows_with(probe: &dyn Fn(&registry::CliSpec) -> CliAuth) -> Vec<LoginRow> {
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
            let (signed_in, install) = match probe(&m.cli) {
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
            let signed_in = match probe(&cli) {
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

/// `/login [provider|n|list]` — bare, hand the host the rows to pick from
/// (`SignIn`; a machine with nothing to offer gets the advice as text);
/// `list`, the table as text; a name or number runs the device sign-in
/// (the caller routes that form as a background task; the poll can wait
/// minutes).
pub(crate) fn login_cmd(
    session: &Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let arg = rest.trim();
    let rows = rows();
    if arg.eq_ignore_ascii_case("list") || (arg.is_empty() && rows.is_empty()) {
        return emit(msg("agent smith", listing(&rows)));
    }
    if arg.is_empty() {
        return emit(PluginEvent::SignIn {
            options: options(&rows),
        });
    }
    match pick(&rows, arg) {
        LoginPick::Device(name) => super::signin::signin_cmd(session, &name, emit),
        LoginPick::Serve(name) => serve(session, &name, emit),
        LoginPick::Note(note) => emit(msg("agent smith", note)),
    }
}

/// A signed-in CLI, picked: pin it, so it serves smith work from the next
/// message — the same pin a model pick writes, so whichever the user chose
/// LAST is the one that serves. The roster goes out again with the new
/// provider so the pane's footer and picker follow.
fn serve(
    session: &Session,
    name: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let note = match crate::credentials::save_pin(name) {
        Ok(()) => format!(
            "\u{2713} {name} now serves smith work \u{2014} signed in through its \
             own CLI (persists across restarts; pick a model or another \
             sign-in to switch)"
        ),
        Err(e) => format!("could not store the {name} pin: {e}"),
    };
    emit(super::rosterev::roster(session.registry().infos()))?;
    emit(msg("agent smith", note))
}

/// `/logout [provider|list]` — remove a stored OAuth grant. With a key
/// still present the provider serves from the key again; with neither it
/// returns to the sign-in affordance. Bare `/logout` hands the host the
/// signed-in rows to pick from (`SignOut`) — never guess which sign-in to
/// destroy; `list` names them as text.
pub(crate) fn logout_cmd(
    session: &Session,
    rest: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let arg = rest.trim();
    let rows = rows();
    let signed: Vec<String> = rows
        .iter()
        .filter(|r| r.device && r.signed_in)
        .map(|r| r.name.clone())
        .collect();
    let name = match (arg.is_empty(), arg.eq_ignore_ascii_case("list")) {
        (true, _) | (_, true) if signed.is_empty() => {
            return emit(msg("agent smith", "no stored sign-in to remove"))
        }
        (true, _) => {
            return emit(PluginEvent::SignOut {
                options: signed_in(&rows),
            })
        }
        (_, true) => {
            return emit(msg(
                "agent smith",
                format!(
                    "signed in: {} \u{2014} /logout <name> picks one",
                    signed.join(", ")
                ),
            ))
        }
        _ => arg.to_string(),
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
    emit(super::rosterev::roster(session.registry().infos()))?;
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
