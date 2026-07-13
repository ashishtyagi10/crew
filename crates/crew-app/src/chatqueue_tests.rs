use super::*;

#[test]
fn stop_bypasses_the_queue_regardless_of_spacing() {
    assert!(is_stop("/stop"));
    assert!(is_stop("  /stop  "));
    assert!(is_stop("/stop #2"));
    assert!(!is_stop("/stopwatch"));
    assert!(!is_stop("hello /stop"));
}
