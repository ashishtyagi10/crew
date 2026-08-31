use super::*;

#[test]
fn first_check_waits_the_launch_delay_then_rearms_six_hourly() {
    let t0 = Instant::now();
    let mut a = AutoUpdate::new(t0);
    assert!(!a.take_due(t0), "not due immediately at launch");
    assert!(!a.take_due(t0 + FIRST_CHECK - Duration::from_secs(1)));
    assert!(a.take_due(t0 + FIRST_CHECK), "due after the launch delay");
    assert!(
        !a.take_due(t0 + FIRST_CHECK),
        "take_due re-arms — not due twice"
    );
    assert!(!a.take_due(t0 + FIRST_CHECK + CHECK_EVERY - Duration::from_secs(1)));
    assert!(a.take_due(t0 + FIRST_CHECK + CHECK_EVERY));
}
