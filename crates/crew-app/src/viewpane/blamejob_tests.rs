use super::*;

#[test]
fn a_fresh_pane_has_no_blame_and_nothing_to_drain() {
    let mut b = Blame::default();
    assert!(b.lines().is_none());
    assert!(b.poll().is_none(), "nothing was asked for");
}

/// A worker that dies without sending must settle the state, not leave the
/// pane waiting forever for something that is not coming.
#[test]
fn a_worker_that_dies_settles_instead_of_waiting_forever() {
    let (tx, rx) = mpsc::channel();
    let mut b = Blame::Loading(rx);
    drop(tx);
    let done = b.poll().expect("a disconnected worker is an answer");
    assert!(done.is_err());
    assert!(matches!(b, Blame::Off), "and the state settled");
    assert!(b.poll().is_none(), "settled means settled");
}

#[test]
fn a_finished_read_becomes_the_lines_the_gutter_labels_with() {
    let (tx, rx) = mpsc::channel();
    let mut b = Blame::Loading(rx);
    assert!(b.poll().is_none(), "still running");
    tx.send(Ok(vec![Line {
        sha: "a1b2c3d".into(),
        author: "Ada".into(),
    }]))
    .unwrap();
    assert!(matches!(b.poll(), Some(Ok(()))));
    assert_eq!(b.lines().map(<[Line]>::len), Some(1));
}

/// A failure puts the state back to Off — asking again is a fresh read, not
/// a retry of a stuck one — and hands the reason up to be said out loud.
#[test]
fn a_failed_read_turns_itself_off_and_says_why() {
    let (tx, rx) = mpsc::channel();
    let mut b = Blame::Loading(rx);
    tx.send(Err("not a file git knows about".into())).unwrap();
    let err = b.poll().expect("an answer").unwrap_err();
    assert_eq!(err, "not a file git knows about");
    assert!(matches!(b, Blame::Off));
}
