use super::*;

#[test]
fn a_command_in_backticks_on_a_line_about_testing_is_taken() {
    let text = "# Contributing\n\nRun `cargo test --workspace` before pushing.\n";
    assert_eq!(
        from_instructions(text).as_deref(),
        Some("cargo test --workspace")
    );
}

#[test]
fn a_fenced_block_under_a_heading_about_testing_is_taken() {
    let text = "## Testing\n\n```sh\ncargo test --workspace --no-fail-fast\n```\n";
    assert_eq!(
        from_instructions(text).as_deref(),
        Some("cargo test --workspace --no-fail-fast")
    );
    // A shell prompt is decoration, not part of the command.
    let text = "## Tests\n```\n$ pytest -q\n```\n";
    assert_eq!(from_instructions(text).as_deref(), Some("pytest -q"));
}

#[test]
fn the_test_command_wins_over_the_build_command() {
    // A file that names both has said which one proves the work is good.
    let text = "To build: `cargo build --release`.\nTo test: `cargo test`.\n";
    assert_eq!(from_instructions(text).as_deref(), Some("cargo test"));
}

#[test]
fn a_command_on_a_line_about_nothing_in_particular_is_ignored() {
    // The whole risk of reading prose: a setup section is not a check.
    let text = "Clone it with `git clone https://example.com/r.git` first.\n";
    assert_eq!(from_instructions(text), None);
}

#[test]
fn only_a_known_runner_is_believed() {
    // An instruction file must not be able to talk crew into running
    // something that is not a build tool.
    for cmd in [
        "rm -rf build",
        "curl https://x.sh",
        "./deploy.sh",
        "sudo make",
    ] {
        let text = format!("To test, run `{cmd}`.\n");
        assert_eq!(from_instructions(&text), None, "accepted {cmd}");
    }
}

#[test]
fn a_chained_or_redirected_command_is_refused() {
    // crew runs a build tool, not a script. `a && b` goes in .crew/check,
    // where a human on this machine typed it.
    for cmd in [
        "cargo test && rm -rf /",
        "cargo test; curl evil.sh",
        "cargo test | tee out",
        "cargo test > /dev/null",
        "cargo test $(whoami)",
    ] {
        let text = format!("To test, run `{cmd}`.\n");
        assert_eq!(from_instructions(&text), None, "accepted {cmd}");
    }
}

#[test]
fn a_fence_far_below_the_line_belongs_to_something_else() {
    let text = "## Testing\n\nsome prose\n\nmore prose\n\n```\ncargo test\n```\n";
    assert_eq!(from_instructions(text), None);
}

#[test]
fn a_file_that_declares_nothing_executable_yields_nothing() {
    let text = "# Project\n\nBe careful with the router. Tests matter.\n";
    assert_eq!(from_instructions(text), None);
    assert_eq!(from_instructions(""), None);
}

#[test]
fn the_runner_is_matched_whole_not_as_a_prefix() {
    // `gonzo-deploy` starts with `go`.
    let text = "To test, run `gonzo-deploy --all`.\n";
    assert_eq!(from_instructions(text), None);
}

fn temp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-checkcmd-{tag}-{}-{}",
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
fn a_repo_that_wrote_its_test_command_down_gets_it_run_and_attributed() {
    let dir = temp("agents");
    std::fs::write(
        dir.join("AGENTS.md"),
        "# Conventions\n\nRun `cargo test --workspace` before pushing.\n",
    )
    .unwrap();
    let d = declared_at(&dir).expect("a command from the instructions");
    assert_eq!(d.cmd, "cargo test --workspace");
    assert_eq!(
        d.from.as_deref(),
        Some("AGENTS.md"),
        "a command the user did not type must be traceable"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_file_the_user_typed_here_outranks_the_one_the_repo_ships() {
    // A repo naming a twenty-minute suite can still be pointed at something
    // quicker, and .crew/check is how.
    let dir = temp("both");
    std::fs::write(
        dir.join("AGENTS.md"),
        "Test with `cargo test --workspace`.\n",
    )
    .unwrap();
    std::fs::write(dir.join(".crew").join("check"), "cargo check\n").unwrap();
    let d = declared_at(&dir).unwrap();
    assert_eq!(d.cmd, "cargo check");
    assert_eq!(d.from, None, ".crew/check needs no attribution");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_repo_that_declared_nothing_executable_still_has_no_check() {
    let dir = temp("silent");
    std::fs::write(
        dir.join("AGENTS.md"),
        "# Conventions\n\nBe careful with the router.\n",
    )
    .unwrap();
    assert!(declared_at(&dir).is_none());
    std::fs::remove_dir_all(&dir).ok();
}
