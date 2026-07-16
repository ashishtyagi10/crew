use super::*;
use std::path::PathBuf;

fn repo(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-standup-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for args in [
        ["init", "-q"].as_slice(),
        &["config", "user.email", "t@t"],
        &["config", "user.name", "t"],
    ] {
        let ok = std::process::Command::new("git")
            .current_dir(&d)
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?}");
    }
    d
}

fn info(name: &str, role: &str) -> crate::AgentInfo {
    crate::AgentInfo {
        name: name.into(),
        role: role.into(),
        model: String::new(),
    }
}

/// `/standup`'s author election (`pick_by_role(&reg.infos(), is_writer)`)
/// must pick the agent whose OWN role advertises a build/writing capability,
/// not the literal name "coder" (no invented specialist is ever named that)
/// and not just the roster's first (arbitrary, LRU-ordered) agent.
/// `standup-scribe` is deliberately NOT first and carries no name hint — only
/// its role says "writes summaries, build notes" — so a fixture where the
/// fallback (`travel-advisor`) coincided with the right answer would prove
/// nothing.
#[test]
fn standup_author_is_elected_by_role_not_by_roster_order() {
    let agents = vec![
        info("travel-advisor", ""),
        info("standup-scribe", "writes summaries, build notes"),
    ];
    assert_eq!(pick_by_role(&agents, is_writer), "standup-scribe");
}

#[test]
fn parse_days_defaults_clamps_and_rejects() {
    assert_eq!(parse_days(""), Some(1));
    assert_eq!(parse_days("  "), Some(1));
    assert_eq!(parse_days("3"), Some(3));
    assert_eq!(parse_days("0"), Some(1), "clamped up");
    assert_eq!(parse_days("500"), Some(MAX_DAYS), "clamped down");
    assert_eq!(parse_days("yesterday"), None, "usage for garbage");
}

#[test]
fn recent_log_reports_commits_none_and_no_repo() {
    let d = repo("log");
    assert_eq!(recent_log(&d, 1).unwrap(), None, "no commits yet");
    std::fs::write(d.join("a.txt"), "x").unwrap();
    for args in [
        ["add", "."].as_slice(),
        &["commit", "-qm", "feat: the overnight thing"],
    ] {
        std::process::Command::new("git")
            .current_dir(&d)
            .args(args)
            .status()
            .unwrap();
    }
    let log = recent_log(&d, 1).unwrap().unwrap();
    assert!(log.contains("feat: the overnight thing"));
    // outside a repo
    let plain = std::env::temp_dir().join(format!("crew-standup-plain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&plain);
    std::fs::create_dir_all(&plain).unwrap();
    assert!(recent_log(&plain, 1).is_err());
}

#[test]
fn standup_prompt_carries_the_log_and_the_ritual() {
    let p = standup_prompt("abc123 feat: shipped it", 2);
    assert!(p.contains("abc123 feat: shipped it"));
    let lower = p.to_lowercase();
    assert!(lower.contains("standup"), "names the format: {p}");
    assert!(lower.contains("first person"), "voice pinned: {p}");
    assert!(
        lower.contains("progress") && lower.contains("risk"),
        "asks for in-progress work and risks: {p}"
    );
    assert!(p.contains('2'), "the window is named");
}
