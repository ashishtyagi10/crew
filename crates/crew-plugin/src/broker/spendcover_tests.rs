//! The cap at places a task spends: the fan's registry agents, a swarm's
//! first ask at $1, and a swarm with the cap off.
use super::kit::{call, run, task, Priced};
use super::*;
use crate::broker::adapter::Usage;
use crate::broker::spendcapwrap::wrap_swarm;
use crate::{Adapter, Registry};
use crew_hive::Provider;
use std::sync::atomic::AtomicUsize;

/// A registry agent that answers at a known price, as an API agent's usage
/// reports one.
struct Scripted {
    name: &'static str,
    micros: u64,
    calls: Arc<AtomicUsize>,
}

impl Adapter for Scripted {
    fn name(&self) -> &str {
        self.name
    }
    fn probe(&self) -> bool {
        true
    }
    fn call(&self, body: &str, t: Duration) -> Result<String, String> {
        self.call_with_usage(body, t).map(|(r, _)| r)
    }
    fn call_with_usage(&self, _body: &str, _t: Duration) -> Result<(String, Usage), String> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let u = Usage {
            cost_microusd: self.micros,
            ..Default::default()
        };
        Ok(("fine\n@done".into(), u))
    }
}

/// A fan of two agents ($6 a call) under a $10 cap already reached; the
/// calls made and the questions asked.
fn fan(yes: bool) -> (usize, usize) {
    let (t, pane) = task(10_000_000, Some(yes));
    t.add(10_000_000);
    let calls = Arc::new(AtomicUsize::new(0));
    let agent = |name| {
        Box::new(Scripted {
            name,
            micros: 6_000_000,
            calls: Arc::clone(&calls),
        }) as Box<dyn Adapter>
    };
    let reg = Registry::new(vec![agent("a"), agent("b")]);
    let _scope = enter(Some(Arc::clone(&t)));
    crate::broker::fan::fan_out(
        &reg,
        &["a".to_string(), "b".to_string()],
        "task",
        Duration::from_secs(5),
        &crate::broker::tick::noop_tick_emit(),
        &mut |_| Ok(()),
    )
    .unwrap();
    let asked = pane.asked.lock().unwrap().len();
    (calls.load(Ordering::Relaxed), asked)
}

#[test]
fn a_fan_over_the_cap_asks_once_and_a_yes_runs_both() {
    assert_eq!(fan(true), (2, 1), "(calls, questions)");
}

#[test]
fn a_fan_over_the_cap_stops_both_on_a_no() {
    assert_eq!(fan(false), (0, 1), "neither agent dialled");
}

#[test]
fn a_fan_counts_what_its_agents_cost() {
    let (t, _pane) = task(100_000_000, Some(true));
    let calls = Arc::new(AtomicUsize::new(0));
    let reg = Registry::new(vec![Box::new(Scripted {
        name: "a",
        micros: 3_000_000,
        calls,
    })]);
    let _scope = enter(Some(Arc::clone(&t)));
    crate::broker::fan::fan_out(
        &reg,
        &["a".to_string()],
        "task",
        Duration::from_secs(5),
        &crate::broker::tick::noop_tick_emit(),
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(t.spent(), 3_000_000);
}

#[test]
fn a_swarm_asks_first_at_one_dollar_and_a_yes_raises_by_the_step() {
    let (t, pane) = task(10_000_000, Some(true));
    let _scope = enter(Some(Arc::clone(&t)));
    let m = Priced::of(600_000);
    let p = wrap_swarm(Arc::clone(&m) as Arc<dyn Provider>, "scripted");
    run(async {
        for _ in 0..3 {
            p.complete(Default::default()).await.unwrap();
        }
    });
    let asked = pane.asked.lock().unwrap();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(
        asked[0].starts_with("spent $1.20 of the $1 cap") && asked[0].contains("(+$10)"),
        "{}",
        asked[0]
    );
    assert_eq!(t.cap.load(Ordering::Relaxed), 11_000_000, "$1 + the step");
}

#[test]
fn a_cap_under_a_dollar_stays_and_a_raised_cap_is_not_lowered_again() {
    let (small, _) = task(500_000, None);
    small.first_ask_at(1_000_000);
    assert_eq!(small.cap.load(Ordering::Relaxed), 500_000);
    let (t, _) = task(10_000_000, None);
    t.first_ask_at(1_000_000);
    t.cap.fetch_add(10_000_000, Ordering::Relaxed); // a yes
    t.first_ask_at(1_000_000); // a second swarm of the same task
    assert_eq!(t.cap.load(Ordering::Relaxed), 11_000_000);
}

#[test]
fn a_swarm_with_the_cap_off_keeps_the_old_governor() {
    // No meter: the provider comes back as it went in, and `backend_at` keeps
    // its $1 `Budget` (`spendcap::current().is_none()`).
    assert!(current().is_none());
    let m: Arc<dyn Provider> = Priced::of(1);
    assert!(Arc::ptr_eq(&m, &wrap_swarm(Arc::clone(&m), "x")));
    let _ = call;
}

#[test]
fn a_gate_asks_from_inside_a_runtime_without_panicking() {
    // The swarm's judge runs inside the swarm's own `block_on`.
    let (t, pane) = task(1_000_000, Some(true));
    t.add(1_000_000);
    let _scope = enter(Some(Arc::clone(&t)));
    let gate = crate::broker::spendgate::Gate::here();
    assert!(run(async { gate.before() }).is_ok());
    assert_eq!(pane.asked.lock().unwrap().len(), 1);
    gate.after(5);
    assert_eq!(t.spent(), 1_000_005);
    // Off, a gate is a no-op.
    drop(_scope);
    assert!(crate::broker::spendgate::Gate::here().before().is_ok());
}
