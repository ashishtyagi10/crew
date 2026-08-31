use super::*;

#[test]
fn the_switch_starts_off_because_a_clean_file_has_nothing_to_show() {
    // Set explicitly rather than assumed: another test may have moved it.
    set(false);
    assert!(!on());
    set(true);
    assert!(on());
    set(false);
}
