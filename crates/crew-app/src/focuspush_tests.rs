use super::has_keys;

#[test]
fn only_the_focused_pane_of_a_focused_window_holds_the_keys() {
    assert!(has_keys(1, 1, Some(true), false));
    assert!(
        has_keys(1, 1, None, false),
        "unknown window focus counts as focused"
    );
    assert!(!has_keys(0, 1, Some(true), false), "another pane");
    assert!(
        !has_keys(1, 1, Some(false), false),
        "the window went to another app"
    );
    assert!(
        !has_keys(1, 1, Some(true), true),
        "the input bar took the keys"
    );
}
