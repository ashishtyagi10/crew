use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn spawn_flag() -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let flag = Arc::new(AtomicBool::new(false));
    let f = Arc::clone(&flag);
    // A thread that runs until its flag trips, so is_finished() is false
    // until we cancel it — lets us test reap/cancel deterministically.
    let h = std::thread::spawn(move || {
        while !f.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
    (flag, h)
}

#[test]
fn register_assigns_increasing_ids() {
    let mut t = Tasks::new();
    let (c1, h1) = spawn_flag();
    let (c2, h2) = spawn_flag();
    let id1 = t.register(Arc::clone(&c1), h1);
    let id2 = t.register(Arc::clone(&c2), h2);
    assert_eq!((id1, id2), (1, 2));
    assert_eq!(t.len(), 2);
    t.cancel_all();
}

#[test]
fn cancel_trips_only_that_task() {
    let mut t = Tasks::new();
    let (c1, h1) = spawn_flag();
    let (c2, h2) = spawn_flag();
    let id1 = t.register(Arc::clone(&c1), h1);
    t.register(Arc::clone(&c2), h2);
    assert!(t.cancel(id1));
    assert!(c1.load(Ordering::Relaxed));
    assert!(!c2.load(Ordering::Relaxed));
    assert!(!t.cancel(999), "unknown id");
    t.cancel_all();
}

#[test]
fn reap_drops_finished_tasks() {
    let mut t = Tasks::new();
    let (c1, h1) = spawn_flag();
    t.register(Arc::clone(&c1), h1);
    c1.store(true, Ordering::Relaxed); // let the thread exit
                                       // Give it a moment, then reap.
    std::thread::sleep(std::time::Duration::from_millis(50));
    t.reap();
    assert_eq!(t.len(), 0);
}

#[test]
fn admit_respects_the_cap() {
    // admit() is count < max; with the default max (>=1) an empty registry admits.
    let t = Tasks::new();
    assert!(t.admit());
}

#[test]
fn admit_is_false_once_at_the_cap() {
    // Register exactly Tasks::max() live tasks and confirm admission is then
    // refused (the "third over cap is rejected" spec requirement). Reads the
    // live cap instead of mutating CREW_MAX_TASKS, so it's safe under the
    // parallel test runner.
    let mut t = Tasks::new();
    let max = Tasks::max();
    for _ in 0..max {
        let (c, h) = spawn_flag();
        t.register(c, h);
    }
    assert_eq!(t.len(), max);
    assert!(!t.admit(), "a registry at the cap must refuse admission");
    t.cancel_all();
}
