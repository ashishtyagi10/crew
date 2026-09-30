//! A steer meeting a swarm: typed during one worker's tool round, it reaches
//! every worker's next request, the host hears once that it joined, and the
//! task remembers it for the lead's closing brief and the judge's (`told`).
use std::sync::atomic::AtomicBool;

use super::tests::{offer, taking, turn, Log};
use super::*;
use crew_hive::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crew_hive::steers::Steers;
use crew_hive::tools::Tools;
use crew_hive::{ApiFactory, StubPlanner};

const SAID: &str = "also check the tests";
const FOLLOW_UP: &str = "TOOL EXCHANGES SO FAR";

/// Every worker asks for one read, then answers; each prompt is recorded.
struct Scripted(Mutex<Vec<String>>);

impl Provider for Scripted {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let text = match req.prompt.contains(FOLLOW_UP) {
            true => "done",
            false => "@tool fs:read {}",
        };
        self.0.lock().unwrap().push(req.prompt);
        let c = Completion {
            text: text.into(),
            input_tokens: 1,
            output_tokens: 1,
            ..Default::default()
        };
        Box::pin(async move { Ok(c) })
    }
}

/// A read during which the user types [`SAID`] — once, under a lock, so no
/// round can end between one worker's read and the offer landing.
struct Reads(Mutex<bool>);

impl Tools for Reads {
    fn hint(&self) -> String {
        "TOOLS: @tool fs:read".into()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        let mut typed = self.0.lock().unwrap();
        if !*typed {
            offer(SAID);
            *typed = true;
        }
        Ok("FILE".into())
    }
}

#[test]
fn every_worker_reads_a_steer_from_the_round_after_it_was_typed() {
    let _env = crate::broker::testenv::mock("unused");
    let _t = turn();
    let log = Log::default();
    let _task = taking(&log);
    let prompts = Arc::new(Scripted(Mutex::default()));
    let factory = ApiFactory::new(Arc::clone(&prompts) as Arc<dyn Provider>, 256)
        .with_tools(Arc::new(Reads(Mutex::new(false))))
        .with_steers(Steers::new(take));
    crate::broker::swarm::run_with_synth(
        "compare the two",
        Arc::new(StubPlanner { fanout: 2 }),
        Arc::new(factory),
        None,
        "",
        Arc::new(AtomicBool::new(false)),
        None,
        None,
        None,
        &mut |_| Ok(()),
    )
    .unwrap();
    let prompts = prompts.0.lock().unwrap().clone();
    let follow_ups: Vec<&String> = prompts.iter().filter(|p| p.contains(FOLLOW_UP)).collect();
    assert!(
        follow_ups.len() >= 2,
        "a round for each worker: {prompts:#?}"
    );
    for p in &follow_ups {
        assert!(
            p.contains(SAID) && p.contains(crew_hive::steers::WORKER_TAIL),
            "{p}"
        );
    }
    assert!(
        prompts.iter().any(|p| !p.contains(SAID)),
        "the first rounds, before it was typed"
    );
    let heard: Vec<String> = log.lock().unwrap().clone();
    assert_eq!(heard, [format!("steered crew: {SAID}")], "announced once");
    assert!(
        told().contains(SAID),
        "kept for the closing brief: {:?}",
        told()
    );
}

#[test]
fn a_new_task_starts_with_nothing_told() {
    let _t = turn();
    let log = Log::default();
    {
        let _first = taking(&log);
        offer(SAID);
        take();
        assert!(told().contains(SAID));
    }
    let _next = taking(&log);
    assert_eq!(
        told(),
        "",
        "the next task's briefs are what they always were"
    );
}
