use super::*;

fn lines(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("line {i}")).collect()
}

#[test]
fn the_slice_is_exactly_the_span() {
    let l = lines(10);
    assert_eq!(slice(&l, 2, 5), "line 2\nline 3\nline 4");
    assert_eq!(slice(&l, 0, 1), "line 0");
}

/// The scrollback the span was measured against may have wrapped away; a
/// range past the end must slice what is left rather than panic.
#[test]
fn a_range_past_the_end_is_clamped() {
    let l = lines(5);
    assert_eq!(slice(&l, 3, 99), "line 3\nline 4");
    assert_eq!(slice(&l, 9, 99), "");
    assert_eq!(
        slice(&l, 4, 2),
        "",
        "a backwards range is empty, not a panic"
    );
    assert_eq!(slice(&[], 0, 5), "");
}

/// One file per pane, named after the command — running `/out` twice in the
/// same pane overwrites its own capture rather than filling the temp dir.
#[test]
fn the_capture_file_is_stable_per_pane_and_command() {
    let a = temp_path(2, "cargo build");
    assert_eq!(a, temp_path(2, "cargo build"));
    assert_ne!(a, temp_path(3, "cargo build"));
    assert_ne!(a, temp_path(2, "npm test"));
    let name = a.file_name().unwrap().to_string_lossy().into_owned();
    assert!(!name.contains(' '), "{name}");
    assert!(!name.contains('/'), "{name}");
    assert_eq!(a.parent(), Some(std::env::temp_dir().as_path()));
}

/// A command whose name is all punctuation still gets a usable filename.
#[test]
fn a_nameless_command_still_gets_a_file() {
    let p = temp_path(0, "///");
    assert!(p.file_name().unwrap().to_string_lossy().contains("out"));
}
