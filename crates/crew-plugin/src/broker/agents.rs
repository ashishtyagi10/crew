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

fn claude() -> CliAdapter {
    CliAdapter {
        name: "claude".into(),
        program: "claude".into(),
        args: vec![
            "-p".into(),
            "{}".into(),
            "--output-format".into(),
            "text".into(),
        ],
        normalize: Normalize::Raw,
    }
}

fn codex() -> CliAdapter {
    CliAdapter {
        name: "codex".into(),
        program: "codex".into(),
        // `--skip-git-repo-check` so it runs outside a repo; prompt as an arg
        // (not stdin) so the session banner stays on stderr.
        args: vec!["exec".into(), "--skip-git-repo-check".into(), "{}".into()],
        normalize: Normalize::Raw,
    }
}

fn opencode() -> CliAdapter {
    CliAdapter {
        name: "opencode".into(),
        program: "opencode".into(),
        args: vec!["run".into(), "--format".into(), "json".into(), "{}".into()],
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
}
