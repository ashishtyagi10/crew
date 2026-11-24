//! The gone-folder look runs off the winit thread: the tick never waits on a stat, one look is
//! in flight at a time, and a finished look lands on a later tick.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::*;

static STATS: AtomicUsize = AtomicUsize::new(0);

/// A stat on a hung mount: slow, then "gone". Bounded, so a look that blocks the tick fails
/// the timing assertion rather than hanging the suite.
fn slow_gone(_: &Path) -> bool {
    STATS.fetch_add(1, Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(400));
    false
}

fn one_folder(tag: &str) -> PathBuf {
    crate::farrecent::clear();
    let d = std::env::temp_dir().join(format!("fargone-{tag}-{}", std::process::id()));
    crate::farrecent::note(&d);
    d
}

#[test]
fn the_tick_starts_a_look_without_waiting_and_lands_it_later() {
    let _g = crate::app::theme_test_guard();
    let dir = one_folder("slow");
    STATS.store(0, Ordering::SeqCst);
    let t = Instant::now();
    assert!(!refresh_with(0, true, slow_gone), "nothing landed yet");
    // Long past the throttle while the first look still runs: no second look starts.
    assert!(!refresh_with(10 * EVERY_MS, true, slow_gone));
    assert!(
        t.elapsed() < Duration::from_millis(200),
        "the tick waited on the stat: {:?}",
        t.elapsed()
    );
    assert!(looking());
    assert!(!is_gone(&dir), "not before the look lands");
    let end = Instant::now() + Duration::from_secs(5);
    let mut landed = false;
    while !landed && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
        // Inside the throttle of the look's start, so landing starts no other look.
        landed = refresh_with(1, true, slow_gone);
    }
    assert!(landed, "the finished look changed the gone set");
    assert!(is_gone(&dir));
    assert_eq!(
        STATS.load(Ordering::SeqCst),
        1,
        "one look in flight at a time"
    );
    crate::farrecent::clear();
}

#[test]
fn a_hidden_section_starts_no_look_and_the_showing_looks_at_once() {
    let _g = crate::app::theme_test_guard();
    let dir = one_folder("hidden");
    assert!(!refresh_with(0, false, |_| false));
    assert!(!looking(), "hidden: no look");
    refresh_with(0, true, |_| false);
    assert!(looking(), "shown: a look at once");
    settle();
    assert!(is_gone(&dir));
    crate::farrecent::clear();
}
