//! What planning cost comes back with the plan: the first ask, a repair
//! re-ask, and an ask that ended in an error all count.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::graph::ModelTier;
use crate::planner::{LlmPlanner, Planner};
use crate::provider::{Completion, CompletionRequest, Provider, ProviderError};
use crate::spent::Spent;

const PLAN: &str = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;

/// One scripted answer: the reply (or a provider error) and its usage.
type Step = (Result<&'static str, &'static str>, u32, u32, u64);

/// Answers from a queue, each reply carrying the usage the script gave it.
struct Billed(Mutex<VecDeque<Step>>);

impl Provider for Billed {
    fn complete(
        &self,
        _req: CompletionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Completion, ProviderError>> + Send>,
    > {
        let (reply, input, output, cost) = self.0.lock().unwrap().pop_front().expect("scripted");
        Box::pin(async move {
            match reply {
                Ok(text) => Ok(Completion {
                    text: text.into(),
                    input_tokens: input,
                    output_tokens: output,
                    cost_microusd: cost,
                    ..Default::default()
                }),
                Err(e) => Err(ProviderError::Api(e.into())),
            }
        })
    }
}

fn planner(steps: &[Step]) -> LlmPlanner<Arc<Billed>> {
    LlmPlanner {
        provider: Arc::new(Billed(Mutex::new(steps.iter().copied().collect()))),
        tier: ModelTier::Standard,
        model: Some("m".into()),
        capabilities: Vec::new(),
    }
}

fn spent(input: u64, output: u64, micros_usd: u64) -> Spent {
    Spent {
        input,
        output,
        micros_usd,
    }
}

#[tokio::test]
async fn a_plan_that_needed_no_repair_costs_its_one_ask() {
    let (plan, cost) = planner(&[(Ok(PLAN), 200, 50, 2000)]).plan_spent("g").await;
    assert_eq!(plan.unwrap().len(), 1);
    assert_eq!(cost, spent(200, 50, 2000));
}

#[tokio::test]
async fn a_repair_re_ask_is_billed_on_top_of_the_ask_it_repaired() {
    let p = planner(&[(Ok("not a plan"), 70, 5, 700), (Ok(PLAN), 200, 50, 2000)]);
    let (plan, cost) = p.plan_spent("g").await;
    assert_eq!(plan.unwrap().len(), 1);
    assert_eq!(cost, spent(270, 55, 2700));
}

#[tokio::test]
async fn a_plan_that_failed_was_still_paid_for() {
    let p = planner(&[(Ok("not a plan"), 70, 5, 700), (Err("overloaded"), 0, 0, 0)]);
    let (plan, cost) = p.plan_spent("g").await;
    assert!(plan.is_err());
    assert_eq!(cost, spent(70, 5, 700), "the first ask happened");
}

#[tokio::test]
async fn plan_alone_still_answers_with_the_graph() {
    let graph = planner(&[(Ok(PLAN), 200, 50, 2000)]).plan("g").await;
    assert_eq!(graph.unwrap().len(), 1);
}
