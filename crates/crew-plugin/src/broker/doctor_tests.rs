use super::*;

fn healthy() -> DoctorInputs {
    DoctorInputs {
        provider: Some("dashscope".into()),
        clis: vec![("claude".into(), true), ("codex".into(), false)],
        bash: true,
        git: true,
        skills: 2,
        plugin_agents: 1,
        mcp_servers: 3,
        memory: Some(120),
        resumable: true,
        sys_tools: true,
        sys_mode: "full",
        turns: 4,
        tokens: 950,
        budget: 0,
    }
}

#[test]
fn render_reports_every_subsystem() {
    let r = render(&healthy());
    assert!(r.contains("✓ provider: dashscope"), "{r}");
    assert!(r.contains("✓ claude") && r.contains("– codex"), "{r}");
    assert!(r.contains("✓ bash"), "{r}");
    assert!(r.contains("✓ git"), "{r}");
    assert!(r.contains("2") && r.to_lowercase().contains("skill"), "{r}");
    assert!(r.contains("3") && r.to_lowercase().contains("mcp"), "{r}");
    assert!(r.to_lowercase().contains("memory"), "{r}");
    assert!(r.to_lowercase().contains("resumable"), "{r}");
    assert!(r.contains("full"), "sys mode shown: {r}");
}

#[test]
fn render_flags_the_broken_bits_with_hints() {
    let mut i = healthy();
    i.provider = None;
    i.bash = false;
    i.git = false;
    let r = render(&i);
    assert!(
        r.contains("✗ provider") && r.contains("DASHSCOPE_API_KEY"),
        "missing provider names the fix: {r}"
    );
    assert!(r.contains("✗ bash"), "{r}");
    assert!(r.contains("✗ git") || r.contains("– git"), "{r}");
}

#[test]
fn absent_extras_read_as_dashes_not_failures() {
    let mut i = healthy();
    i.skills = 0;
    i.mcp_servers = 0;
    i.memory = None;
    i.resumable = false;
    let r = render(&i);
    // no ✗ for optional surfaces that simply aren't configured
    for l in r.lines().filter(|l| l.starts_with('✗')) {
        assert!(
            !l.contains("skill") && !l.contains("mcp") && !l.contains("memory"),
            "optional surface marked broken: {l}"
        );
    }
}

#[test]
fn on_path_finds_only_executables() {
    let d = std::env::temp_dir().join(format!("crew-doctor-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let exe = d.join("mytool");
    std::fs::write(&exe, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(d.join("notexec"), "data").unwrap();
    let path = format!("/nonexistent:{}", d.display());
    assert!(on_path("mytool", &path));
    assert!(!on_path("notexec", &path));
    assert!(!on_path("missing", &path));
}

/// Inherited from the deleted `/status`, whose tests these were: the turn
/// count, the token total, the per-turn average and the relay budget lived
/// nowhere else in the product, so they had to land somewhere before that
/// construct could go.
#[test]
fn the_session_line_carries_what_status_used_to() {
    let r = render(&healthy());
    assert!(r.contains("\u{2713} session: 4 turns"), "{r}");
    assert!(r.contains("~950 tok"), "{r}");
    assert!(r.contains("~237/turn"), "{r}");
    assert!(r.contains("unlimited budget"), "{r}");
}

#[test]
fn a_single_turn_is_singular_and_a_set_budget_is_shown() {
    let mut i = healthy();
    i.turns = 1;
    i.budget = 50_000;
    let r = render(&i);
    assert!(r.contains("1 turn \u{00b7}"), "{r}");
    assert!(!r.contains("1 turns"), "{r}");
    assert!(r.contains("~50000 tok budget"), "{r}");
}

/// A `0/turn` reading would be meaningless, and the division would panic.
#[test]
fn no_turns_yet_means_no_rate() {
    let mut i = healthy();
    i.turns = 0;
    let r = render(&i);
    assert!(r.contains("session: 0 turns"), "{r}");
    assert!(!r.contains("/turn)"), "no turns yet, no rate to show: {r}");
}

/// A keyless machine is not a broken one any more — the advice must say so.
#[test]
fn a_missing_provider_offers_the_keyless_route_first() {
    let mut i = healthy();
    i.provider = None;
    let r = render(&i);
    assert!(r.contains("sign in to claude"), "{r}");
    assert!(r.contains("OPENROUTER_API_KEY"), "{r}");
}
