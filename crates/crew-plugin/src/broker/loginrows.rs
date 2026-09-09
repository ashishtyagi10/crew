//! The pure half of `/login`: the rows a machine offers, the listing text,
//! and what a number or name picks — no keychain, no CLI probe, so every
//! case is table-testable. `logincmd` gathers the live rows and runs picks.
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
        let login = r.cli_login.unwrap_or_default();
        let detail = match (r.signed_in, r.install, r.key_present) {
            (true, _, _) => "\u{2713} signed in (vendor CLI)".to_string(),
            (false, Some(install), _) => {
                format!("not installed \u{00b7} `{install}`, then `{login}`")
            }
            (false, None, true) => format!(
                "key present \u{00b7} `{login}` signs in with OAuth instead \
                 (the sign-in then serves)"
            ),
            (false, None, false) => format!("signed out \u{00b7} run `{login}`"),
        };
        lines.push(format!(" \u{25cb} {} \u{2014} {detail}", r.name));
    }
    lines.join("\n")
}

/// The rows as a host sees them (`PluginEvent::SignIn`): device flows
/// first — those are the ones a pick can run — then the CLI-owned ones with
/// their command. Pure.
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
    match (r.cli_login, r.install, r.signed_in) {
        // Already signed in through the CLI: the pick is a status, not a
        // second sign-in — a user who has just run the login and picks the
        // row to check must not be told to run it again.
        (Some(_), _, true) => {
            let out = registry::by_name(&r.name)
                .and_then(|e| e.mint)
                .map(|m| format!(" \u{2014} `{}` signs out", m.logout))
                .unwrap_or_default();
            LoginPick::Note(format!(
                "{} is signed in through its own CLI and serving{out}",
                r.name
            ))
        }
        (Some(login), Some(install), false) => LoginPick::Note(format!(
            "{} signs in through its own CLI, which crew can't find on its PATH \
             \u{2014} `{install}` if it isn't installed, then `{login}`; \
             then /login again here",
            r.name
        )),
        (Some(login), None, false) => LoginPick::Note(format!(
            "{} signs in through its own CLI \u{2014} run `{login}`, then /login \
             again here (crew re-checks by itself within a minute)",
            r.name
        )),
        (None, _, _) => LoginPick::Device(r.name.clone()),
    }
}
