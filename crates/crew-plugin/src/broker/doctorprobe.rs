//! `/doctor`'s probing half: everything that touches the live environment
//! (PATH, git, disk, the session's MCP host and counters). Split from
//! `doctor` so the render stays pure and the file under the line cap.
use std::path::Path;

use super::doctor::DoctorInputs;
use crew_hive::childproc::no_console_window;

/// Is `bin` an executable on the `:`-separated `path`?
pub(crate) fn on_path(bin: &str, path: &str) -> bool {
    path.split(':').filter(|d| !d.is_empty()).any(|d| {
        let p = Path::new(d).join(bin);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&p)
                .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            p.is_file()
        }
    })
}

/// One integration's line: its tools, and whether the credential it names is actually set.
/// A manifest whose token is missing works perfectly until its first call, and this is the
/// place that can say so before then.
fn integration_line(i: &super::integration::Integration) -> String {
    use super::integration::Auth;
    let env = match &i.auth {
        Auth::Bearer { env } | Auth::Header { env, .. } | Auth::Query { env, .. } => Some(env),
        Auth::None => None,
    };
    let auth = match env {
        None => "no credential needed".to_string(),
        Some(e) => match std::env::var(e).ok().filter(|v| !v.trim().is_empty()) {
            Some(_) => format!("{e} is set"),
            None => format!("{e} is NOT set \u{2014} calls will refuse"),
        },
    };
    format!("\u{2013} {}: {} tool(s), {auth}", i.name, i.tools.len())
}

/// Probe the live environment for the report. `session` supplies the counters
/// that only it knows (turns, tokens); everything else is probed here.
pub(crate) fn gather(session: &super::session::Session) -> DoctorInputs {
    use std::sync::atomic::Ordering;
    let path = std::env::var("PATH").unwrap_or_default();
    let active = super::discover::resolved_provider();
    // A delegated subscription outranks keys in live resolution, and it is
    // the one provider `resolved_provider` (API-shaped) cannot see — so the
    // provider line states it, degradation included, instead of reading
    // "none" on a perfectly serviceable machine.
    let provider = match super::auth::resolved_live() {
        super::auth::Resolved::Delegated { name, agent } => Some(format!(
            "{name} subscription (via the {agent} CLI \u{2014} swarm planning degrades to the relay)"
        )),
        _ => active.map(|p| p.name().to_string()),
    };
    DoctorInputs {
        // The same resolution the roster is built from, so `/doctor` reports a
        // stored key's provider rather than only an exported one's.
        provider,
        auth: {
            let mut v: Vec<_> = super::auth::state::snapshot()
                .iter()
                .map(auth_line)
                .collect();
            // Where OAuth grants live (condition 6): keychain when `security`
            // answers, the 0600 file everywhere else — stated, not guessed.
            let kc = super::auth::keychain::bin().is_some();
            let mark = if kc { '✓' } else { '–' };
            v.push((
                mark,
                "token store".into(),
                super::auth::tokens::backend_note().into(),
            ));
            v
        },
        others: super::discover::configured_providers()
            .into_iter()
            .filter(|n| Some(n.as_str()) != active.map(|p| p.name()))
            .collect(),
        // From the adapter table, not a name list — the CLIs `/doctor`
        // reports are exactly the ones the roster can drive.
        clis: super::agents::known_adapters()
            .iter()
            .map(|a| (a.name().to_string(), on_path(a.name(), &path)))
            .collect(),
        bash: Path::new("/bin/bash").exists(),
        git: no_console_window(&mut std::process::Command::new("git"))
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .is_ok_and(|o| o.status.success()),
        skills: super::skills::load().len(),
        plugin_agents: super::plugins::load().len(),
        sidecar: std::env::var("CREW_SIDECAR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|raw| {
                let runnable = crew_hive::worker::stdio::parse_command(&raw)
                    .is_some_and(|(p, _)| crew_hive::worker::stdio::probe(&p));
                (raw, runnable)
            }),
        integrations: super::integration::load()
            .iter()
            .map(integration_line)
            .collect(),
        mcp_servers: crate::mcp::config::load().len(),
        mcp_detail: if crate::mcp::config::load().is_empty() {
            Vec::new()
        } else {
            session
                .lock_mcp()
                .report()
                .lines()
                .map(String::from)
                .collect()
        },
        memory: super::memory::load().map(|m| m.len()),
        resumable: super::sessionlog::tail().is_some(),
        sys_tools: super::systools::enabled(),
        sys_mode: super::systools::mode_label(),
        turns: session.turns.load(Ordering::Relaxed),
        tokens: session.tokens.load(Ordering::Relaxed),
        budget: super::session::token_budget(),
    }
}

/// One provider's report line: STATE words only — a key value cannot reach
/// this function, let alone leave it.
fn auth_line(p: &super::auth::state::ProviderInfo) -> (char, String, String) {
    use super::auth::state::AuthState;
    let detail = match p.state {
        AuthState::SignedIn => "signed in (subscription)".to_string(),
        AuthState::SignedOut => format!(
            "signed out \u{2014} sign in: run `{}`",
            p.login.unwrap_or_default()
        ),
        AuthState::KeyPresent => "key present".to_string(),
        AuthState::NoKey => "no key".to_string(),
        AuthState::Installed => "installed (carries its own sign-in)".to_string(),
        AuthState::Absent => "not installed".to_string(),
    };
    let healthy = matches!(
        p.state,
        AuthState::SignedIn | AuthState::KeyPresent | AuthState::Installed
    );
    (if healthy { '✓' } else { '–' }, p.name.clone(), detail)
}
