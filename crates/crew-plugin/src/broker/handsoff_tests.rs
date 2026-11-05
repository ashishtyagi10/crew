use super::*;
use crate::broker::claudeagent::ClaudeAgent;

fn argv(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn each_known_cli_runs_a_review_in_its_read_only_mode() {
    let codex = cli_args("codex", argv(&["exec", "--skip-git-repo-check", "body"]));
    assert_eq!(
        codex,
        argv(&[
            "exec",
            "--sandbox",
            "read-only",
            "--skip-git-repo-check",
            "body"
        ])
    );
    let opencode = cli_args(
        "/opt/bin/opencode",
        argv(&["run", "--format", "json", "body"]),
    );
    assert_eq!(
        opencode,
        argv(&["run", "--agent", "plan", "--format", "json", "body"])
    );
}

#[test]
fn a_manifest_agent_is_left_as_its_author_wrote_it() {
    let args = argv(&["--ask", "body"]);
    assert_eq!(cli_args("mytool", args.clone()), args);
}

#[test]
fn claude_code_reviews_in_plan_mode_and_works_with_its_hands_otherwise() {
    let mut a = ClaudeAgent::new(None);
    assert!(!a.args("b", None).iter().any(|x| x == "plan"));
    a.read_only = true;
    let v = a.args("b", None);
    let at = v.iter().position(|x| x == "--permission-mode").unwrap();
    assert_eq!(v[at + 1], "plan", "{v:?}");
}
