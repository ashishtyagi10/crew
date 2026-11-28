//! The cap through a whole relay hop: held between tool rounds, never inside
//! one, and a no leaves a turn that settled the way `/stop` does.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::kit::{calling, hop, said, Reply};
use crate::broker::spendcap::{enter, Ask, Say, TaskSpend};

/// A round that costs `micros`, calling the file tool.
fn round(micros: u64) -> Reply {
    let mut c = calling("fs__read", serde_json::json!({"path": "a.rs"}));
    c.cost_microusd = micros;
    Ok(c)
}

/// A task capped at $10 whose asker answers `yes` at once; the questions
/// asked, the lines said and the task's stop.
fn capped(
    yes: bool,
) -> (
    Arc<TaskSpend>,
    Arc<Mutex<Vec<String>>>,
    Arc<Mutex<Vec<String>>>,
    Arc<AtomicBool>,
) {
    let (asked, said): (Arc<Mutex<Vec<String>>>, Arc<Mutex<Vec<String>>>) =
        (Arc::default(), Arc::default());
    let (stop, a, s) = (
        Arc::new(AtomicBool::new(false)),
        Arc::clone(&asked),
        Arc::clone(&said),
    );
    let ask: Ask = Arc::new(move |id, q| {
        a.lock().unwrap().push(q.to_string());
        crate::broker::approval::deliver_answer(id, yes);
        true
    });
    let say: Say = Arc::new(move |l| s.lock().unwrap().push(l.to_string()));
    let t = TaskSpend::new(10_000_000, Arc::clone(&stop), ask, say).unwrap();
    (t.waiting(Duration::from_secs(5)), asked, said, stop)
}

#[test]
fn a_no_between_rounds_stops_the_hop_before_the_next_call() {
    let (t, asked, said_, stop) = capped(false);
    let _scope = enter(Some(t));
    let turn = hop(
        true,
        vec![
            round(6_000_000),
            round(6_000_000),
            round(6_000_000),
            Ok(said("late\n@done")),
        ],
        8,
        Arc::clone(&stop),
    );
    assert_eq!(turn.seen.len(), 2, "the third call never went out");
    assert_eq!(
        turn.ran.len(),
        2,
        "the round that crossed the cap ran whole"
    );
    assert_eq!(asked.lock().unwrap().len(), 1);
    assert_eq!(
        *said_.lock().unwrap(),
        vec!["stopped at the spend cap ($12)".to_string()]
    );
    assert!(
        turn.done()
            .is_some_and(|d| d.contains("stopped \u{2014} 2 tool calls made")),
        "settled as a stop does: {:?}",
        turn.done()
    );
    assert!(stop.load(Ordering::Relaxed));
}

#[test]
fn a_yes_goes_on_and_the_turn_finishes() {
    let (t, asked, said_, stop) = capped(true);
    let _scope = enter(Some(t));
    let turn = hop(
        true,
        vec![
            round(6_000_000),
            round(6_000_000),
            round(6_000_000),
            Ok(said("fine\n@done")),
        ],
        8,
        stop,
    );
    assert_eq!(turn.seen.len(), 4);
    assert_eq!(asked.lock().unwrap().len(), 1);
    assert!(said_.lock().unwrap().is_empty());
    assert!(turn.done().is_some());
}
