//! The built-in agent adapters. Invocations were verified against the installed
//! CLIs: claude `-p … --output-format text` and `codex exec` both print the
//! reply on stdout (codex's banner goes to stderr, discarded by the runner);
//! opencode emits a JSON event stream. To add a fourth agent, write one more
//! constructor here and push it into [`known_adapters`] — the broker is
//! untouched.
use super::adapter::{Adapter, CliAdapter, Normalize};

/// Every agent the broker knows how to drive. Discovery keeps only the ones
/// whose CLI is actually installed (see [`super::Registry::discover`]).
pub fn known_adapters() -> Vec<Box<dyn Adapter>> {
    vec![Box::new(claude()), Box::new(codex()), Box::new(opencode())]
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
