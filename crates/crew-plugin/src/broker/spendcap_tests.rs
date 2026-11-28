//! The spend cap: pause between calls, ask once, a yes raises, a no ends.
//! Answers are delivered from inside the injected asker, so no emitter is
//! installed (`approval::set_emitter` is process-wide) and nothing waits on a
//! clock except the tests that mean to.
use super::kit::{call, capped, run, task, Priced};
use super::*;

#[test]
fn under_the_cap_nothing_pauses() {
    let (t, pane) = task(10_000_000, Some(true));
    let m = Priced::of(1_000_000);
    let c = capped(&m, &t);
    run(async {
        for _ in 0..9 {
            call(&c).await.unwrap();
        }
    });
    assert_eq!(m.made(), 9);
    assert!(pane.asked.lock().unwrap().is_empty(), "asked under the cap");
    assert_eq!(t.spent(), 9_000_000);
}

#[test]
fn crossing_the_cap_asks_once_between_calls_and_a_yes_raises_it() {
    let (t, pane) = task(10_000_000, Some(true));
    let m = Priced::of(5_000_000);
    let c = capped(&m, &t);
    run(async {
        // Two calls reach $10; the third is the one held, never the second.
        for _ in 0..3 {
            call(&c).await.unwrap();
        }
    });
    assert_eq!(m.made(), 3, "a yes lets the call through");
    let asked = pane.asked.lock().unwrap();
    assert_eq!(asked.len(), 1, "asked once: {asked:?}");
    assert!(
        asked[0].starts_with("spent $10 of the $10 cap") && asked[0].contains("continues (+$10)"),
        "{}",
        asked[0]
    );
    assert!(pane.said.lock().unwrap().is_empty());
    assert!(!pane.cancel.load(Ordering::Relaxed));
    assert_eq!(
        t.cap.load(Ordering::Relaxed),
        20_000_000,
        "raised by one step"
    );
}

#[test]
fn a_no_ends_the_task_with_one_line_and_asks_no_more() {
    let (t, pane) = task(10_000_000, Some(false));
    let m = Priced::of(5_200_000);
    let c = capped(&m, &t);
    let out = run(async {
        call(&c).await.unwrap();
        call(&c).await.unwrap(); // $10.40
        let held = call(&c).await;
        let again = call(&c).await;
        (held, again)
    });
    assert!(out.0.is_err() && out.1.is_err());
    assert_eq!(m.made(), 2, "no call after the no");
    assert_eq!(pane.asked.lock().unwrap().len(), 1);
    assert_eq!(
        *pane.said.lock().unwrap(),
        vec!["stopped at the spend cap ($10.40)".to_string()]
    );
    assert!(pane.cancel.load(Ordering::Relaxed), "the task is stopped");
}

#[test]
fn silence_is_a_no() {
    let (t, pane) = task(1_000_000, None);
    t.add(1_000_000);
    let c = capped(&Priced::of(1), &t);
    assert!(run(call(&c)).is_err());
    assert_eq!(pane.said.lock().unwrap().len(), 1);
    assert!(pane.cancel.load(Ordering::Relaxed));
}

#[test]
fn stop_while_paused_ends_it_without_the_cap_line() {
    let (t, pane) = task(1_000_000, None);
    t.add(1_000_000);
    let c = capped(&Priced::of(1), &t);
    let flag = Arc::clone(&pane.cancel);
    let started = Instant::now();
    let held = run(async {
        let stopper = async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            flag.store(true, Ordering::Relaxed);
        };
        tokio::join!(call(&c), stopper).0
    });
    assert!(held.is_err());
    assert!(started.elapsed() < Duration::from_millis(140), "waited out");
    assert!(
        pane.said.lock().unwrap().is_empty(),
        "a stop is not the cap"
    );
}

#[test]
fn agents_of_a_swarm_share_one_cap_and_one_question() {
    let (t, pane) = task(10_000_000, Some(true));
    let (a, b) = (Priced::of(4_000_000), Priced::of(7_000_000));
    let (ca, cb) = (capped(&a, &t), capped(&b, &t));
    run(async {
        // Each agent alone is under $10; together they are over, and the
        // two that arrive at once open one question between them.
        call(&ca).await.unwrap();
        call(&cb).await.unwrap();
        let (x, y) = tokio::join!(call(&ca), call(&cb));
        x.unwrap();
        y.unwrap();
    });
    assert_eq!(
        pane.asked.lock().unwrap().len(),
        1,
        "one question for the task"
    );
    assert_eq!(a.made() + b.made(), 4);
}
