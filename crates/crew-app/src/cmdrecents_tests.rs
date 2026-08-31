use super::*;

fn reset() {
    set(Vec::new());
}

/// A command run twice is one entry that moved, not two — otherwise the
/// cap fills with repeats of one habit and the list stops being a summary
/// of anything.
#[test]
fn running_the_same_command_twice_moves_it_rather_than_duplicating() {
    let _g = crate::app::motion_test_guard();
    reset();
    record("/theme");
    record("/gradient");
    let list = record("/theme");
    assert_eq!(list, vec!["/theme", "/gradient"]);
    reset();
}

/// The cap holds, and it drops the OLDEST — a cap that dropped the newest
/// would make the most recent command the one thing never remembered.
#[test]
fn the_cap_holds_and_forgets_the_oldest_first() {
    let _g = crate::app::motion_test_guard();
    reset();
    for i in 0..MAX + 5 {
        record(&format!("/c{i}"));
    }
    let list = now();
    assert_eq!(list.len(), MAX);
    assert_eq!(list[0], format!("/c{}", MAX + 4), "newest leads");
    assert!(!list.contains(&"/c0".to_string()), "oldest was forgotten");
    reset();
}

/// An unrun command has to sort after every run one, or the tie-break
/// becomes a filter that hides most of the table.
#[test]
fn anything_unrun_sorts_last() {
    let list = vec!["/theme".to_string(), "/font".to_string()];
    assert_eq!(rank_of(&list, "/theme"), 0);
    assert_eq!(rank_of(&list, "/font"), 1);
    assert_eq!(rank_of(&list, "/never-run"), usize::MAX);
}

/// A restored list longer than the cap (an old config, a hand-edited one)
/// must not smuggle a longer history past it.
#[test]
fn a_restored_list_is_capped_too() {
    let _g = crate::app::motion_test_guard();
    set((0..MAX + 7).map(|i| format!("/c{i}")).collect());
    assert_eq!(now().len(), MAX);
    reset();
}
