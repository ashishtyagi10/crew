//! The reach against a real repository (the stale-callers kit): a changed
//! `add` called from three production files reaches three callers; a test's
//! call is not counted; a definition nothing calls adds nothing to the head.
use super::{reach, reached};
use crate::broker::stalecallers::tests::{edit, kit, repo, LIB3};

#[test]
fn a_changed_function_reaches_its_callers_tests_not_counted() {
    let dir = kit("reach");
    std::fs::write(dir.join("lib_tests.rs"), "fn t() {\n    add(1, 2);\n}\n").unwrap();
    let patch = edit(&dir, "lib.rs", LIB3);
    assert_eq!(reach(&dir, &patch), Some((3, 3)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nothing_calls_it_nothing_is_said() {
    let dir = repo("alone", &[("lib.rs", "fn lonely(a: u8) {}\n")]);
    let patch = edit(&dir, "lib.rs", "fn lonely(a: u8, b: u8) {}\n");
    assert_eq!(reach(&dir, &patch), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_reach_rides_the_head_before_its_hint() {
    let hint = "1 file changed: ~ lib.rs \u{2014} /diff shows them";
    assert_eq!(
        reached(hint, Some((1, 1))),
        "1 file changed: ~ lib.rs \u{b7} reaches 1 caller in 1 file \u{2014} /diff shows them"
    );
    assert_eq!(
        reached("2 files changed: ~ a.rs, ~ b.rs", Some((6, 3))),
        "2 files changed: ~ a.rs, ~ b.rs \u{b7} reaches 6 callers in 3 files"
    );
    assert_eq!(
        reached("1 file changed: ~ a.rs", None),
        "1 file changed: ~ a.rs"
    );
}
