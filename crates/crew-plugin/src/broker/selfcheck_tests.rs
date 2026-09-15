use super::*;

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-selfcheck-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(dir.join(".crew")).unwrap();
    dir
}

#[test]
fn the_command_is_the_first_real_line_of_the_declared_file() {
    let dir = temp("declared");
    std::fs::write(
        dir.join(".crew").join("check"),
        "# the project's gate\n\ncargo test --workspace\nnever reached\n",
    )
    .unwrap();
    assert_eq!(command_at(&dir).as_deref(), Some("cargo test --workspace"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_project_with_no_declared_check_has_none() {
    let dir = temp("none");
    assert_eq!(command_at(&dir), None);
    std::fs::write(dir.join(".crew").join("check"), "# only a comment\n").unwrap();
    assert_eq!(command_at(&dir), None, "a comment is not a command");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn what_a_project_probably_checks_with_is_read_off_its_marker_file() {
    let dir = temp("detect");
    assert_eq!(detected_at(&dir), None);
    std::fs::write(dir.join("Cargo.toml"), "[package]\n").unwrap();
    assert_eq!(detected_at(&dir), Some("cargo check --workspace"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_command_that_ran_and_failed_is_a_failure_even_though_the_shell_worked() {
    // `sys:run` answers Ok("exit 3\n…") for a command that ran and failed;
    // reading the Result alone would call that a pass.
    let o = outcome(Ok("exit 3\nerror[E0308]: mismatched types\n".into()));
    assert!(!o.ok, "a non-zero exit read as a pass");
    let said = line("cargo check", &o);
    assert!(said.contains("FAILED"), "{said}");
    assert!(said.contains("error[E0308]"), "{said}");
    assert!(!said.contains("exit 3"), "the exit line is noise: {said}");
}

#[test]
fn a_clean_run_is_one_line_and_a_shell_that_could_not_start_is_a_failure() {
    assert_eq!(
        line("cargo check", &outcome(Ok("exit 0\nFinished\n".into()))),
        "check: cargo check — passed"
    );
    let o = outcome(Err("spawn /bin/sh: no such file".into()));
    assert!(!o.ok && line("x", &o).contains("FAILED"));
}

#[test]
fn only_the_head_of_a_long_failure_is_shown() {
    let long: String = (0..50).map(|i| format!("error {i}\n")).collect();
    let said = line("make", &outcome(Ok(format!("exit 1\n{long}"))));
    assert_eq!(said.lines().count(), 1 + FAIL_LINES);
}

#[test]
fn nothing_is_said_when_a_task_changed_nothing() {
    let session = Session::default();
    let mut said = Vec::new();
    let mut emit = |ev: PluginEvent| {
        if let PluginEvent::Message { text, .. } = &ev {
            said.push(text.clone());
        }
        Ok(())
    };
    after_task(&session, false, &mut emit).unwrap();
    assert!(said.is_empty());
}

#[test]
fn the_tip_is_said_once_and_only_when_it_can_name_the_command() {
    assert!(note("cargo check --workspace").contains(".crew/check"));
    assert!(note("cargo check --workspace").contains("cargo check --workspace"));
}
