use super::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

fn t0() -> Instant {
    Instant::now()
}

#[test]
fn the_first_warm_goes_out() {
    assert!(Latch::default().admit(t0(), false));
}

#[test]
fn a_second_warm_inside_twenty_seconds_does_not() {
    let (mut l, t) = (Latch::default(), t0());
    assert!(l.admit(t, false));
    assert!(!l.admit(t + Duration::from_secs(1), false));
    assert!(!l.admit(t + EVERY - Duration::from_millis(1), false));
}

#[test]
fn twenty_seconds_on_it_warms_again() {
    let (mut l, t) = (Latch::default(), t0());
    assert!(l.admit(t, false));
    assert!(l.admit(t + EVERY, false));
    // …and the window restarts from that one, not the first.
    assert!(!l.admit(t + EVERY + Duration::from_secs(1), false));
}

/// A running task holds the connection already. Being turned away for it
/// must not spend the window, or the first key after the task ends is lost.
#[test]
fn a_busy_broker_does_not_warm_and_does_not_spend_the_window() {
    let (mut l, t) = (Latch::default(), t0());
    assert!(!l.admit(t, true));
    assert!(l.admit(t + Duration::from_secs(1), false));
}

#[test]
fn crew_prewarm_zero_turns_it_off_and_nothing_else_does() {
    assert!(!enabled(Some("0")));
    assert!(enabled(None));
    assert!(enabled(Some("1")));
    assert!(enabled(Some("")));
}

/// Counts the warms it is asked for.
struct Counting(Arc<AtomicUsize>);

impl crew_hive::Provider for Counting {
    fn complete(
        &self,
        _req: crew_hive::CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<crew_hive::Completion, crew_hive::ProviderError>> + Send>>
    {
        Box::pin(async { Ok(crew_hive::Completion::default()) })
    }

    fn warm(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn counting() -> (Arc<dyn crew_hive::Provider>, Arc<AtomicUsize>) {
    let n = Arc::new(AtomicUsize::new(0));
    (Arc::new(Counting(Arc::clone(&n))), n)
}

#[test]
fn the_resolved_provider_is_the_one_warmed() {
    let (p, n) = counting();
    assert!(warm_resolved(Some((p, "qwen-plus".into()))));
    assert_eq!(n.load(Ordering::SeqCst), 1);
}

#[test]
fn the_mock_and_no_provider_warm_nothing() {
    let (p, n) = counting();
    assert!(!warm_resolved(Some((p, "mock".into()))));
    assert!(!warm_resolved(None));
    assert_eq!(n.load(Ordering::SeqCst), 0);
}

/// Through real discovery: the GUI harness's mock broker and a keyless one
/// both resolve to nothing worth a socket.
#[test]
fn discovery_under_the_mock_or_with_no_key_warms_nothing() {
    {
        let _env = crate::broker::testenv::mock("hi");
        assert!(!warm_next_provider());
    }
    let _env = crate::broker::testenv::no_provider();
    assert!(!warm_next_provider());
}
