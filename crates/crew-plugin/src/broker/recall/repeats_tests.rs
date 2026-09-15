use super::*;

const NOW: u64 = 1_700_000_000_000;
const DAY: u64 = 24 * 60 * 60 * 1000;

/// A graph holding one remembered check verdict, exactly as `selfcheck`
/// writes it: `check: <cmd>` asked, `failed: <digest>` answered.
fn remembered(cmd: &str, verdict: &str, when: u64) -> Graph {
    let mut g = Graph::default();
    let id = g.touch(
        Kind::Turn,
        &format!("t{when}"),
        &format!("you asked: check: {cmd}\ncrew answered: {verdict}"),
        when,
    );
    let f = g.touch(
        Kind::File,
        "crates/crew-app/src/nav.rs",
        "crates/crew-app/src/nav.rs",
        when,
    );
    g.link(id, f, super::super::node::Rel::Mentions);
    g
}

#[test]
fn the_shells_exit_line_is_not_part_of_the_failure() {
    let d = digest("exit 101\n\nerror[E0308]: mismatched types\n  --> src/a.rs:12:5\n");
    assert!(d.starts_with("error[E0308]"), "{d}");
    assert!(!d.contains("exit"), "the exit code is not the cause: {d}");
}

#[test]
fn the_same_error_at_a_different_line_is_the_same_failure() {
    let before = digest("exit 1\nerror[E0308]: mismatched types\n --> src/a.rs:12:5\n");
    let after = digest("exit 1\nerror[E0308]: mismatched types\n --> src/a.rs:481:9\n");
    assert_ne!(before, after, "the two outputs really do differ");
    assert_eq!(
        signature(&before),
        signature(&after),
        "a line number moved and the failure was called a new one"
    );
}

#[test]
fn a_different_error_is_a_different_failure() {
    let a = signature(&digest("exit 1\nerror[E0308]: mismatched types\n"));
    let b = signature(&digest("exit 1\nerror[E0425]: cannot find value `x`\n"));
    assert_ne!(a, b);
}

#[test]
fn the_stored_prefix_never_counts_as_part_of_the_signature() {
    // What the graph holds is `failed: <digest>`; what the query has in hand
    // is the digest alone. They must sign the same.
    let d = digest("exit 2\nerror: linker failed\n");
    assert_eq!(signature(&format!("failed: {d}")), signature(&d));
}

#[test]
fn the_first_failure_of_its_kind_is_not_a_repeat() {
    let g = remembered("cargo test", "passed", NOW - DAY);
    assert!(prior(&g, "cargo test", "exit 1\nerror: boom\n", 2).is_none());
}

#[test]
fn a_failure_the_graph_has_seen_before_carries_when_and_where() {
    let g = remembered(
        "cargo test",
        "failed: error[E0308]: mismatched types --> crates/crew-app/src/nav.rs:12",
        NOW - 2 * DAY,
    );
    let p = prior(
        &g,
        "cargo test",
        "exit 1\nerror[E0308]: mismatched types\n --> crates/crew-app/src/nav.rs:97\n",
        2,
    )
    .expect("the same failure was not recognised");
    assert_eq!(p.times, 1);
    assert_eq!(p.last_ms, NOW - 2 * DAY);
    assert_eq!(p.files, vec!["crates/crew-app/src/nav.rs".to_string()]);
    let s = sentence(&p, "2d ago");
    assert!(s.contains("2d ago") && s.contains("2nd time"), "{s}");
    assert!(s.contains("crates/crew-app/src/nav.rs"), "{s}");
}

#[test]
fn another_commands_failure_is_not_this_commands_history() {
    let g = remembered("npm test", "failed: error: boom", NOW - DAY);
    assert!(
        prior(&g, "cargo test", "exit 1\nerror: boom\n", 2).is_none(),
        "one command's breakage was read as another's"
    );
}

#[test]
fn an_empty_failure_matches_nothing_rather_than_everything() {
    let g = remembered("cargo test", "failed: ", NOW - DAY);
    assert!(
        prior(&g, "cargo test", "exit 1\n\n", 2).is_none(),
        "a failure with no text matched a stored one with no text"
    );
}

#[test]
fn the_teens_get_their_suffix_right() {
    let said: Vec<String> = [2, 3, 4, 11, 12, 13, 21, 22]
        .iter()
        .map(|n| nth(*n))
        .collect();
    assert_eq!(
        said,
        ["2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd"]
    );
}
