//! The pure half of the sign-in rows (`/model <provider>`, `/logout`): the
//! rows a machine offers, their picker form, and what a name picks — no
//! keychain, no CLI probe, so every case is table-testable. `logincmd`
//! gathers the live rows and runs picks.
use super::auth::registry;

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
    /// An API key is present alongside — worth naming, because a key used
    /// to hide the sign-in affordance entirely.
    pub key_present: bool,
    /// The CLI that owns this sign-in is not installed: how to get it.
    pub install: Option<&'static str>,
}

/// The rows as a host sees them (the `Roster` event's `signins`): device
/// flows first — those are the ones a pick can run — then the CLI-owned
/// ones with their command. Pure.
pub(crate) fn options(rows: &[LoginRow]) -> Vec<crate::SignInOption> {
    let opt = |r: &LoginRow| crate::SignInOption {
        name: r.name.clone(),
        device: r.device,
        signed_in: r.signed_in,
        key_present: r.key_present,
        login: r.cli_login.map(str::to_string),
        install: r.install.map(str::to_string),
        logout: registry::by_name(&r.name)
            .and_then(|e| e.mint)
            .map(|m| m.logout.to_string()),
    };
    rows.iter()
        .filter(|r| r.device)
        .chain(rows.iter().filter(|r| !r.device))
        .map(opt)
        .collect()
}

/// The rows `/logout` can act on (`PluginEvent::SignOut`): every signed-in
/// provider — the grants crew removes first, then the CLI-owned sign-ins
/// (a pick there only names the CLI's own sign-out). Pure.
pub(crate) fn signed_in(rows: &[LoginRow]) -> Vec<crate::SignInOption> {
    options(rows).into_iter().filter(|o| o.signed_in).collect()
}

/// What `/model <provider>` resolves to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LoginPick {
    /// Run this provider's device flow (a signed-in pick re-authenticates).
    Device(String),
    /// A CLI-owned sign-in that is live: the pick makes it SERVE — the
    /// pin moves there, as picking a model moves it (last choice wins).
    Serve(String),
    /// A message for the pane.
    Note(String),
}

/// Resolve a provider name (numbers are `/model <n>`'s, resolved by
/// `modelpick` against its own listing). Pure.
pub(crate) fn pick(rows: &[LoginRow], arg: &str) -> LoginPick {
    let Some(r) = rows.iter().find(|r| r.name.eq_ignore_ascii_case(arg)) else {
        return LoginPick::Note(match registry::by_name(arg) {
            Some(e) => format!(
                "{} offers no sign-in flow \u{2014} paste a key via /model",
                e.name
            ),
            None => format!(
                "unknown provider \u{201c}{arg}\u{201d} \u{2014} /model lists who can sign in"
            ),
        });
    };
    match (r.cli_login, r.install, r.signed_in) {
        // Already signed in through the CLI: the pick makes it serve — a
        // user who has just run the login and picks the row means "use
        // this one", never "tell me to run the login again".
        (Some(_), _, true) => LoginPick::Serve(r.name.clone()),
        (Some(login), Some(install), false) => LoginPick::Note(format!(
            "{} signs in through its own CLI, which crew can't find on its PATH \
             \u{2014} `{install}` if it isn't installed, then `{login}`; \
             then /model {} again here",
            r.name, r.name
        )),
        (Some(login), None, false) => LoginPick::Note(format!(
            "{} signs in through its own CLI \u{2014} run `{login}`, then /model {} \
             again here (crew re-checks by itself within a minute)",
            r.name, r.name
        )),
        // A live grant, picked, serves; signing in again is `/logout` first.
        (None, _, true) => LoginPick::Serve(r.name.clone()),
        (None, _, false) => LoginPick::Device(r.name.clone()),
    }
}
