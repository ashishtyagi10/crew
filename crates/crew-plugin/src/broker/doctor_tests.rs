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
