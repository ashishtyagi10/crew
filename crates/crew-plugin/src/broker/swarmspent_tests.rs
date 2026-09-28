//! A swarm turn's one total is every call the turn made: the router that
//! chose the swarm, the plan and its repair re-ask, each worker, the closing
//! answer and the judge. Each part runs on its real path over a provider
//! that bills what the script says, and the total must come out exact.
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crew_hive::{
    ApiFactory, Completion, CompletionRequest, LlmPlanner, ModelTier, Provider, ProviderError,
    Spent,
};

use super::swarmanswer::SynthFn;
use super::swarmverify::Judge;
use crate::broker::intent::decision::Decision;
use crate::broker::intent::{routed, Shape, SpentClassifier};
use crate::broker::session::Session;
use crate::broker::testenv;
use crate::protocol::PluginEvent;

/// A reply and its usage: text, tokens in, tokens out, micro-USD.
type Step = (&'static str, u32, u32, u64);

const PLAN: &str = r#"[{"id":0,"title":"left","prompt":"look left","deps":[]},
{"id":1,"title":"right","prompt":"look right","deps":[]}]"#;
const ROUTER: Spent = Spent {
    input: 100,
    output: 10,
    micros_usd: 1_000,
};
const PLANNED: Step = (PLAN, 200, 50, 2_000);
const WORKER: Step = ("clear", 300, 100, 3_000);
const ANSWER: Step = ("Both ways are clear.", 400, 200, 4_000);
const JUDGE: Spent = Spent {
    input: 150,
    output: 20,
    micros_usd: 1_500,
};

/// Answers from a script, each reply billed as scripted; the last step
/// repeats, so the two workers can share one.
struct Billed(Mutex<VecDeque<Step>>);

impl Billed {
    fn new(steps: &[Step]) -> Arc<Self> {
        Arc::new(Billed(Mutex::new(steps.iter().copied().collect())))
    }
}

impl Provider for Billed {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let mut q = self.0.lock().unwrap();
        let (text, input, output, cost) = match q.len() {
            1 => *q.front().unwrap(),
            _ => q.pop_front().expect("scripted"),
        };
        Box::pin(async move {
            Ok(Completion {
                text: text.into(),
                input_tokens: input,
                output_tokens: output,
                cost_microusd: cost,
                ..Default::default()
            })
        })
    }
}

/// `(tokens, tok_in, tok_out, cost)` of every turn total the turn said.
fn totals(evs: &[PluginEvent]) -> Vec<(u64, u64, u64, u64)> {
    evs.iter()
        .filter_map(|e| match e {
            PluginEvent::Stats {
                agent,
                tokens,
                tok_in,
                tok_out,
                cost_microusd,
                ..
            } if agent.is_empty() => Some((*tokens, *tok_in, *tok_out, *cost_microusd)),
            _ => None,
        })
        .collect()
}

/// One routed swarm turn: a costed router, then the swarm on a planner over
/// `planning`, two workers, the streamed closing answer and `judge`.
fn turn(planning: &[Step], judge: Option<&SynthFn>) -> Vec<PluginEvent> {
    let _env = testenv::mock("unused");
    let router = |_: &str| Ok::<_, String>(("SHAPE: swarm".to_string(), ROUTER));
    let planner = Arc::new(LlmPlanner {
        provider: Billed::new(planning),
        tier: ModelTier::Standard,
        model: Some("m".into()),
        capabilities: Vec::new(),
    });
    let factory = Arc::new(ApiFactory::new(Billed::new(&[WORKER]), 2048));
    let answer = super::swarmstream::over(Billed::new(&[ANSWER]), "m".into(), 2048);
    let mut arm =
        |d: &Decision, _: &mut Session, emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>| {
            assert_eq!(d.shape, Shape::Swarm);
            #[rustfmt::skip]
        let ran = super::run_with_synth(
            "look both ways", planner.clone(), factory.clone(), None, "m",
            Arc::new(AtomicBool::new(false)), None, Some(&*answer),
            judge.map(Judge::new), emit,
        );
            ran.map(|_| ())
        };
    let mut evs = Vec::new();
    let mut keep = |ev: PluginEvent| -> anyhow::Result<()> {
        evs.push(ev);
        Ok(())
    };
    let router = Some(&router as SpentClassifier);
    routed(
        "look both ways",
        router,
        &mut Session::new(),
        &mut arm,
        &mut keep,
    )
    .unwrap();
    evs
}

#[test]
fn the_turn_total_is_router_plan_workers_and_answer() {
    let evs = turn(&[PLANNED], None);
    let (tok_in, tok_out) = (100 + 200 + 2 * 300 + 400, 10 + 50 + 2 * 100 + 200);
    let cost = 1_000 + 2_000 + 2 * 3_000 + 4_000;
    assert_eq!(
        totals(&evs),
        vec![(tok_in + tok_out, tok_in, tok_out, cost)],
        "one total, every call in it: {evs:?}"
    );
}

#[test]
fn a_judge_s_call_is_in_the_total() {
    let judge = |_: &str| Ok::<_, String>(("MET: both clear".to_string(), JUDGE));
    let evs = turn(&[PLANNED], Some(&judge));
    let (tok_in, tok_out) = (1_300 + 150, 460 + 20);
    assert_eq!(
        totals(&evs),
        vec![(tok_in + tok_out, tok_in, tok_out, 13_000 + 1_500)],
        "{evs:?}"
    );
}

#[test]
fn a_planner_repair_re_ask_is_in_the_total() {
    let evs = turn(&[("Sure! Here is my plan.", 70, 5, 700), PLANNED], None);
    let (tok_in, tok_out) = (1_300 + 70, 460 + 5);
    assert_eq!(
        totals(&evs),
        vec![(tok_in + tok_out, tok_in, tok_out, 13_000 + 700)],
        "{evs:?}"
    );
}
