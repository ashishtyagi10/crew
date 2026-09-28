use std::future::pending;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;

#[tokio::test]
async fn work_that_ends_first_is_kept() {
    let flag = AtomicBool::new(false);
    assert_eq!(unless(&flag, async { 7 }).await, Some(7));
}

#[tokio::test]
async fn a_flag_already_set_never_polls_the_work() {
    let flag = AtomicBool::new(true);
    let polled = AtomicBool::new(false);
    let out = unless(&flag, async {
        polled.store(true, Ordering::SeqCst);
    })
    .await;
    assert_eq!(out, None);
    assert!(
        !polled.load(Ordering::SeqCst),
        "the work ran after the stop"
    );
}

#[tokio::test]
async fn a_flag_set_while_waiting_drops_the_work_within_a_tick_or_two() {
    let flag = Arc::new(AtomicBool::new(false));
    let setter = Arc::clone(&flag);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        setter.store(true, Ordering::Relaxed);
    });
    let started = Instant::now();
    let out: Option<()> = unless(&flag, pending()).await;
    assert_eq!(out, None);
    let took = started.elapsed();
    assert!(took < Duration::from_millis(30) + TICK * 3, "took {took:?}");
}
