use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

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
    let id1 = t.register("a".into(), Arc::clone(&c1), h1, Instant::now());
    let id2 = t.register("b".into(), Arc::clone(&c2), h2, Instant::now());
    assert_eq!((id1, id2), (1, 2));
    assert_eq!(t.len(), 2);
    t.cancel_all();
}

#[test]
fn cancel_trips_only_that_task() {
    let mut t = Tasks::new();
    let (c1, h1) = spawn_flag();
    let (c2, h2) = spawn_flag();
    let id1 = t.register("a".into(), Arc::clone(&c1), h1, Instant::now());
    t.register("b".into(), Arc::clone(&c2), h2, Instant::now());
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
    t.register("a".into(), Arc::clone(&c1), h1, Instant::now());
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
fn describe_lists_id_and_label() {
    let mut t = Tasks::new();
    let (c1, h1) = spawn_flag();
    t.register("refactor".into(), Arc::clone(&c1), h1, Instant::now());
    let d = t.describe(Instant::now());
    assert_eq!(d.len(), 1);
    assert!(d[0].contains("#1") && d[0].contains("refactor"), "{}", d[0]);
    t.cancel_all();
}
