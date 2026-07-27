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
mod tests {
    use super::*;

    #[test]
    fn known_adapters_are_the_three_agents() {
        let names: Vec<String> = known_adapters().iter().map(|a| a.name().into()).collect();
        assert_eq!(names, vec!["claude", "codex", "opencode"]);
    }

    #[test]
    fn claude_args_carry_body_and_text_format() {
        let c = claude();
        assert_eq!(c.program, "claude");
        assert!(c.args.contains(&"-p".to_string()));
        assert!(c.args.contains(&"text".to_string()));
    }

    #[test]
    fn codex_skips_git_repo_check() {
        assert!(codex().args.contains(&"--skip-git-repo-check".to_string()));
    }

    #[test]
    fn role_for_known_and_unknown() {
        assert!(!role_for("codex").is_empty());
        assert!(!role_for("claude").is_empty());
        assert_eq!(role_for("nope"), "");
    }

    /// A fake already-registered agent, so the dedupe rule can be tested
    /// without building a provider-backed specialist.
    struct Taken(&'static str);
    impl Adapter for Taken {
        fn name(&self) -> &str {
            self.0
        }
        fn probe(&self) -> bool {
            true
        }
        fn call(&self, _: &str, _: std::time::Duration) -> Result<String, String> {
            Ok(String::new())
        }
    }

    fn names(agents: &[Box<dyn Adapter>]) -> Vec<String> {
        agents.iter().map(|a| a.name().to_string()).collect()
    }

    #[test]
    fn installed_clis_join_an_empty_roster() {
        let mut agents: Vec<Box<dyn Adapter>> = Vec::new();
        append_where(&mut agents, |_| true);
        assert_eq!(names(&agents), vec!["claude", "codex", "opencode"]);
    }

    /// The case this whole feature exists for is a machine with SOME of them.
    #[test]
    fn only_installed_clis_are_added() {
        let mut agents: Vec<Box<dyn Adapter>> = Vec::new();
        append_where(&mut agents, |a| a.name() == "codex");
        assert_eq!(names(&agents), vec!["codex"]);
    }

    #[test]
    fn no_installed_cli_leaves_the_roster_alone() {
        let mut agents: Vec<Box<dyn Adapter>> = Vec::new();
        append_where(&mut agents, |_| false);
        assert!(agents.is_empty());
    }

    /// An explicit local declaration outranks the built-in of the same name:
    /// a user who wrote a `claude` manifest meant THAT one.
    #[test]
    fn an_existing_agent_keeps_its_name() {
        let mut agents: Vec<Box<dyn Adapter>> = vec![Box::new(Taken("claude"))];
        append_where(&mut agents, |_| true);
        assert_eq!(names(&agents), vec!["claude", "codex", "opencode"]);
        assert_eq!(agents[0].role(), "planning, analysis, prose");
        // …and it is still the caller's adapter, not a fresh CliAdapter.
        assert!(agents[0].call("x", std::time::Duration::from_secs(1)) == Ok(String::new()));
    }

    #[test]
    fn the_name_match_is_case_insensitive() {
        let mut agents: Vec<Box<dyn Adapter>> = vec![Box::new(Taken("Codex"))];
        append_where(&mut agents, |_| true);
        assert_eq!(names(&agents), vec!["Codex", "claude", "opencode"]);
    }

    #[test]
    fn with_model_appends_only_when_set() {
        let base = vec!["-p".to_string(), "{}".to_string()];
        assert_eq!(
            with_model(base.clone(), "--model", Some("haiku".into())),
            vec!["-p", "{}", "--model", "haiku"]
        );
        assert_eq!(with_model(base.clone(), "--model", None), base);
        assert_eq!(
            with_model(base.clone(), "--model", Some(String::new())),
            base
        );
    }
}
