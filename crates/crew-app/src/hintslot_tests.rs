/// A mode opened on one test thread is not on another's pane 0: the leak
/// that stamped hint labels over `helpscrim_tests`' pane while
/// `hintact_tests` held the mode open.
#[test]
fn a_mode_open_on_one_thread_is_not_seen_on_another() {
    let rows = vec!["see https://example.com/x".chars().collect::<Vec<char>>()];
    assert!(
        crate::hints::open(0, &rows, &[], Vec::new()),
        "it opened here"
    );
    let elsewhere = std::thread::spawn(|| {
        let mut cells = Vec::new();
        (
            crate::hints::active(),
            crate::hints::mark_pane(&mut cells, 0),
        )
    })
    .join()
    .unwrap();
    assert!(crate::hints::active(), "still open on this thread");
    crate::hints::close();
    assert_eq!(elsewhere, (false, false), "another thread saw it");
}
