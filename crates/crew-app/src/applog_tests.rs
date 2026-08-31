use super::*;

#[test]
fn drain_returns_queued_items_in_order_then_empties() {
    let log = AppLog::default();
    log.sender().send((LogLevel::Info, "a".into())).unwrap();
    log.sender().send((LogLevel::Error, "b".into())).unwrap();
    let drained = log.drain();
    assert_eq!(
        drained,
        vec![
            (LogLevel::Info, "a".to_string()),
            (LogLevel::Error, "b".to_string())
        ]
    );
    assert!(log.drain().is_empty(), "a second drain finds nothing");
}

#[test]
fn a_thread_can_send_through_a_cloned_sender() {
    let log = AppLog::default();
    let tx = log.sender();
    std::thread::spawn(move || {
        tx.send((LogLevel::Info, "from thread".into())).unwrap();
    })
    .join()
    .unwrap();
    assert_eq!(
        log.drain(),
        vec![(LogLevel::Info, "from thread".to_string())]
    );
}
