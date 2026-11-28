//! The cap over a swarm: the plan's agents spend into one meter, whichever
//! agent crosses the cap is the one held, and a no stops the whole run.
use super::*;
use crate::broker::spendcap::{Ask, Say, TaskSpend};
use crate::broker::spendcapwrap::Capped;
use crate::broker::testenv;
use crew_hive::StubPlanner;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Reply = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<crew_hive::Completion, crew_hive::ProviderError>>
            + Send,
    >,
>;

/// Every call is a finished answer that cost $6.
struct Dear(Arc<AtomicUsize>);

impl crew_hive::Provider for Dear {
    fn complete(&self, _req: crew_hive::CompletionRequest) -> Reply {
        self.0.fetch_add(1, Ordering::Relaxed);
        Box::pin(async {
            Ok(crew_hive::Completion {
                text: "done".into(),
                cost_microusd: 6_000_000,
                ..Default::default()
            })
        })
    }
}

/// The plan's three tasks (two leaves, a merge) at $6 a call under a $10 cap
/// whose asker says `yes`: the model calls made, the questions asked, the
/// lines said, and whether the run was stopped.
fn swarm(yes: bool) -> (usize, usize, Vec<String>, bool) {
    let _env = testenv::mock("unused");
    let (asked, said) = (
        Arc::new(AtomicUsize::new(0)),
        Arc::new(Mutex::new(Vec::new())),
    );
    let (stop, a, s) = (
        Arc::new(AtomicBool::new(false)),
        Arc::clone(&asked),
        Arc::clone(&said),
    );
    let ask: Ask = Arc::new(move |id, _| {
        a.fetch_add(1, Ordering::Relaxed);
        crate::broker::approval::deliver_answer(id, yes);
        true
    });
    let say: Say = Arc::new(move |l| s.lock().unwrap().push(l.to_string()));
    let meter = TaskSpend::new(10_000_000, Arc::clone(&stop), ask, say)
        .unwrap()
        .waiting(Duration::from_secs(5));
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = Arc::new(Capped {
        inner: Arc::new(Dear(Arc::clone(&calls))),
        model: "m".into(),
        task: meter,
    });
    let factory = Arc::new(crew_hive::ApiFactory::new(provider, 256));
    run_with(
        "do three things",
        Arc::new(StubPlanner { fanout: 2 }),
        factory,
        None,
        "test-model",
        Arc::clone(&stop),
        None,
        &mut |_| Ok(()),
    )
    .unwrap();
    let said = said.lock().unwrap().clone();
    (
        calls.load(Ordering::Relaxed),
        asked.load(Ordering::Relaxed),
        said,
        stop.load(Ordering::Relaxed),
    )
}

#[test]
fn agents_spend_into_one_cap_and_a_yes_lets_the_run_finish() {
    let (calls, asked, said, stopped) = swarm(true);
    assert_eq!((calls, asked), (3, 1), "three agents, one question");
    assert!(said.is_empty() && !stopped);
}

#[test]
fn a_no_stops_the_run_at_the_agent_that_crossed_the_cap() {
    let (calls, asked, said, stopped) = swarm(false);
    assert_eq!((calls, asked), (2, 1), "the merge never spent");
    assert_eq!(said, vec!["stopped at the spend cap ($12)".to_string()]);
    assert!(stopped);
}

/// The closing call (`swarmstream::over`) at a cap already reached, with the
/// task's gate taken on this thread as `swarmanswer::live` takes it: the
/// provider's dials made, the questions asked and the answer's result.
fn closing(yes: bool) -> (usize, usize, bool) {
    use crate::broker::spendcap::{enter, kit::Priced};
    let _env = testenv::mock("unused");
    let asked = Arc::new(AtomicUsize::new(0));
    let a = Arc::clone(&asked);
    let ask: Ask = Arc::new(move |id, _| {
        a.fetch_add(1, Ordering::Relaxed);
        crate::broker::approval::deliver_answer(id, yes);
        true
    });
    let meter = TaskSpend::new(10_000_000, Arc::default(), ask, Arc::new(|_| {}))
        .unwrap()
        .waiting(Duration::from_secs(5));
    meter.add(10_000_000);
    let _scope = enter(Some(Arc::clone(&meter)));
    let model = Priced::of(2_000_000);
    let answer = swarmstream::over(
        Arc::clone(&model) as Arc<dyn crew_hive::Provider>,
        "m".into(),
        256,
        crate::broker::spendgate::Gate::here(),
    );
    let out = answer("brief", &mut |_| {});
    (model.made(), asked.load(Ordering::Relaxed), out.is_ok())
}

#[test]
fn the_closing_call_at_the_cap_asks_first_and_a_yes_dials_once() {
    assert_eq!(closing(true), (1, 1, true));
}

#[test]
fn the_closing_call_at_the_cap_never_dials_on_a_no() {
    assert_eq!(closing(false), (0, 1, false));
}
