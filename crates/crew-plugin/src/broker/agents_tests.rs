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
