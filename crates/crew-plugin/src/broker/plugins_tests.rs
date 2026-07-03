use super::*;

fn manifest(name: &str, command: &str, args: &[&str], role: &str) -> Manifest {
    Manifest {
        name: name.into(),
        command: command.into(),
        args: args.iter().map(|s| s.to_string()).collect(),
        role: role.into(),
    }
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "crew-plugins-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn from_manifest_normalizes_name_and_keeps_role() {
    let a = from_manifest(manifest("My Agent", "sh", &["-c", "{}"], "greps fast")).unwrap();
    assert_eq!(a.name(), "my-agent");
    assert_eq!(a.role(), "greps fast");
}

#[test]
fn from_manifest_appends_missing_placeholder() {
    let a = from_manifest(manifest("x", "prog", &["run"], "")).unwrap();
    assert_eq!(a.cli.args, vec!["run", "{}"]);
    let b = from_manifest(manifest("y", "prog", &["--msg", "{}"], "")).unwrap();
    assert_eq!(b.cli.args, vec!["--msg", "{}"]);
}

#[test]
fn from_manifest_rejects_blank_name_or_command() {
    assert!(from_manifest(manifest("  ", "prog", &[], "")).is_none());
    assert!(from_manifest(manifest("x", "  ", &[], "")).is_none());
}

#[test]
fn plugin_agent_probe_and_call_use_the_cli() {
    let a = from_manifest(manifest(
        "echoer",
        "sh",
        &["-c", "printf %s \"$0\"", "{}"],
        "",
    ))
    .unwrap();
    assert!(a.probe());
    assert_eq!(
        a.call("hi", std::time::Duration::from_secs(5)).unwrap(),
        "hi"
    );
}

#[test]
fn load_dir_reads_sorted_json_and_skips_garbage() {
    let d = tmpdir("loaddir");
    std::fs::write(
        d.join("b.json"),
        r#"{"name":"beta","command":"sh","role":"second"}"#,
    )
    .unwrap();
    std::fs::write(d.join("a.json"), r#"{"name":"alpha","command":"sh"}"#).unwrap();
    std::fs::write(d.join("bad.json"), "not json").unwrap();
    std::fs::write(d.join("note.md"), "ignored").unwrap();
    let agents = load_dir(&d);
    let names: Vec<&str> = agents.iter().map(|a| a.name()).collect();
    assert_eq!(names, vec!["alpha", "beta"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn load_dir_sees_manifests_added_after_a_previous_scan() {
    // Hot-reload contract: manifests re-read the directory on every message,
    // so a plugin agent added while crew runs joins the roster without a
    // restart.
    let d = tmpdir("fresh");
    std::fs::write(d.join("a.json"), r#"{"name":"a","command":"sh"}"#).unwrap();
    assert_eq!(load_dir(&d).len(), 1);
    std::fs::write(d.join("b.json"), r#"{"name":"b","command":"sh"}"#).unwrap();
    let names: Vec<String> = load_dir(&d).iter().map(|p| p.name().to_string()).collect();
    assert_eq!(names, vec!["a", "b"]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn load_dir_of_missing_path_is_empty() {
    assert!(load_dir(Path::new("/nonexistent/xyz")).is_empty());
}

#[test]
fn append_skips_taken_names_and_uninstalled_commands() {
    struct Stub;
    impl Adapter for Stub {
        fn name(&self) -> &str {
            "taken"
        }
        fn probe(&self) -> bool {
            true
        }
        fn call(&self, _b: &str, _t: std::time::Duration) -> Result<String, String> {
            Ok(String::new())
        }
    }
    let mut agents: Vec<Box<dyn Adapter>> = vec![Box::new(Stub)];
    let shadow = from_manifest(manifest("Taken", "sh", &[], "")).unwrap();
    let missing = from_manifest(manifest("ghost", "no-such-binary-xyz", &[], "")).unwrap();
    let ok = from_manifest(manifest("fine", "sh", &[], "")).unwrap();
    for p in [shadow, missing, ok] {
        let taken = agents
            .iter()
            .any(|a| a.name().eq_ignore_ascii_case(p.name()));
        if !taken && p.probe() {
            agents.push(Box::new(p));
        }
    }
    let names: Vec<&str> = agents.iter().map(|a| a.name()).collect();
    assert_eq!(names, vec!["taken", "fine"]);
}
