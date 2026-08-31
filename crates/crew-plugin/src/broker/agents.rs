//! The built-in agent adapters. Invocations were verified against the installed
//! CLIs: claude `-p … --output-format text` and `codex exec` both print the
//! reply on stdout (codex's banner goes to stderr, discarded by the runner);
//! opencode emits a JSON event stream. To add a fourth agent, write one more
//! constructor here and push it into [`known_adapters`] — the broker is
//! untouched.
use super::adapter::{Adapter, CliAdapter, Normalize};

/// Every agent the broker knows how to drive. Discovery keeps only the ones
/// whose CLI is actually installed (see [`append_installed`]).
pub fn known_adapters() -> Vec<Box<dyn Adapter>> {
    vec![Box::new(claude()), Box::new(codex()), Box::new(opencode())]
}

/// Append every known CLI agent that is actually installed on this machine.
///
/// These need NO API key: `claude`, `codex` and `opencode` each carry their
/// own sign-in, so a user who has already logged into one of them has a
/// working crew roster without giving crew a credential at all. That was true
/// of the code long before it was true of the product — [`known_adapters`]
/// existed, was exported, and had no caller anywhere, so a machine with all
/// three CLIs installed and authenticated still opened the pane on "No agents
/// available. Set OPENROUTER_API_KEY…". This is the caller.
///
/// Runs AFTER manifest plugin agents so a user's own `.crew/agents/` manifest
/// wins the name: an explicit local declaration should always beat a built-in.
pub(crate) fn append_installed(agents: &mut Vec<Box<dyn Adapter>>) {
    append_where(agents, |a| a.probe());
}

/// [`append_installed`] with the installed-check injected, so the dedupe rule
/// is testable on a machine regardless of which CLIs it happens to have.
fn append_where(agents: &mut Vec<Box<dyn Adapter>>, installed: impl Fn(&dyn Adapter) -> bool) {
    for a in known_adapters() {
        let taken = agents
            .iter()
            .any(|x| x.name().eq_ignore_ascii_case(a.name()));
        if !taken && installed(a.as_ref()) {
            agents.push(a);
        }
    }
}

/// A short capability hint per known *external CLI* agent, surfaced in the peer
/// list so an agent hands the task off to the right one. Empty for anything
/// else — API specialists are invented at runtime and carry their own role
/// (see `ApiAdapter::role`), so there is nothing static to look up.
pub fn role_for(name: &str) -> &'static str {
    match name {
        // External CLI agents (still selectable via the CLI adapters).
        "claude" => "planning, analysis, prose",
        "codex" => "building, implementation",
        "opencode" => "review, second opinion",
        _ => "",
    }
}

/// Append a model-selection flag when `model` is set, so a cost-conscious user
/// can point an agent at a cheaper model (e.g. `CREW_CLAUDE_MODEL=...`) with no
/// code change. Pure (caller passes the env value) so it's testable.
fn with_model(mut args: Vec<String>, flag: &str, model: Option<String>) -> Vec<String> {
    if let Some(m) = model.filter(|m| !m.is_empty()) {
        args.push(flag.into());
        args.push(m);
    }
    args
}

fn claude() -> CliAdapter {
    CliAdapter {
        name: "claude".into(),
        program: "claude".into(),
        args: with_model(
            vec![
                "-p".into(),
                "{}".into(),
                "--output-format".into(),
                "text".into(),
            ],
            "--model",
            std::env::var("CREW_CLAUDE_MODEL").ok(),
        ),
        normalize: Normalize::Raw,
    }
}

fn codex() -> CliAdapter {
    CliAdapter {
        name: "codex".into(),
        program: "codex".into(),
        // `--skip-git-repo-check` so it runs outside a repo; prompt as an arg
        // (not stdin) so the session banner stays on stderr.
        args: with_model(
            vec!["exec".into(), "--skip-git-repo-check".into(), "{}".into()],
            "-m",
            std::env::var("CREW_CODEX_MODEL").ok(),
        ),
        normalize: Normalize::Raw,
    }
}

fn opencode() -> CliAdapter {
    CliAdapter {
        name: "opencode".into(),
        program: "opencode".into(),
        args: with_model(
            vec!["run".into(), "--format".into(), "json".into(), "{}".into()],
            "-m",
            std::env::var("CREW_OPENCODE_MODEL").ok(),
        ),
        normalize: Normalize::OpencodeJson,
    }
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod tests;
