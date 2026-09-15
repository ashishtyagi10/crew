use super::*;

use crate::broker::recall;

/// A directory that is not a git repository, so `dirty` contributes nothing
/// and the test is about memory alone.
fn nowhere() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-opening-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_tree_crew_has_never_worked_in_says_nothing() {
    let session = Session::default();
    let dir = nowhere();
    assert_eq!(line(&session, &dir), None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_line_says_what_we_were_doing_and_how_long_ago() {
    let session = Session::default();
    recall::lock(&session.recall).record("finish the recall graph", "shipped in v0.22.23");
    let dir = nowhere();
    let said = line(&session, &dir).expect("a line");
    assert!(
        said.starts_with("last time (just now): finish the recall graph"),
        "{said}"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_failing_check_is_the_thing_you_most_need_to_know_on_sitting_down() {
    let session = Session::default();
    {
        let mut r = recall::lock(&session.recall);
        r.record("finish the recall graph", "shipped");
        r.record("check: cargo test", "failed: 2 tests broke");
    }
    let dir = nowhere();
    let said = line(&session, &dir).expect("a line");
    assert!(said.contains("the check was FAILING"), "{said}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_passing_check_says_so_quietly() {
    let session = Session::default();
    recall::lock(&session.recall).record("check: cargo test", "passed");
    let dir = nowhere();
    let said = line(&session, &dir).expect("a line");
    assert!(said.contains("the check was passing"), "{said}");
    assert!(!said.contains("FAILING"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_long_request_is_clipped_rather_than_wrapped_across_the_pane() {
    let session = Session::default();
    recall::lock(&session.recall).record(&"x".repeat(400), "done");
    let dir = nowhere();
    let said = line(&session, &dir).expect("a line");
    assert!(
        said.chars().count() < ASKED_CAP + 60,
        "{} chars",
        said.chars().count()
    );
    assert!(said.contains("clipped"), "{said}");
    std::fs::remove_dir_all(&dir).ok();
}
