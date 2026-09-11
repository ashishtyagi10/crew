use std::sync::Arc;

use super::billed;
use crate::agent::{AgentContext, AgentFactory};
use crate::apiagent::ApiFactory;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::pricing::cost_microusd;
use crate::provider::{Completion, MockProvider};

fn reply(input: u32, output: u32) -> Completion {
    Completion {
        input_tokens: input,
        output_tokens: output,
        ..Default::default()
    }
}

#[test]
fn a_known_model_bills_at_its_own_price_not_the_tiers() {
    let got = billed("claude-haiku-4-5", ModelTier::Standard, &reply(100, 10));
    assert_eq!(got, 150, "haiku: 100 × $1 + 10 × $5 per Mtok");
    assert_ne!(got, cost_microusd(ModelTier::Standard.model_id(), 100, 10));
}

#[test]
fn an_unknown_model_bills_at_the_tiers_own_model_price_not_zero() {
    let got = billed("mock", ModelTier::Cheap, &reply(100, 10));
    assert_eq!(got, cost_microusd(ModelTier::Cheap.model_id(), 100, 10));
    assert_eq!(got, 150, "haiku: 100 × $1 + 10 × $5 per Mtok");
    // The list, not a second table: Opus at its list price.
    assert_eq!(billed("mock", ModelTier::Capable, &reply(1, 1)), 5 + 25);
}

#[test]
fn a_cost_the_provider_reported_outranks_every_estimate() {
    let mut c = reply(1_000, 1_000);
    c.cost_microusd = 7;
    assert_eq!(billed("claude-sonnet-4-6", ModelTier::Standard, &c), 7);
}

/// The whole path: a factory pinned to a Haiku id — what `swarmconf::backend`
/// does — must publish a Haiku-priced `CostDelta` for a task the planner left
/// at `Standard`, not a Sonnet-priced one.
#[tokio::test]
async fn a_factory_pinned_to_haiku_bills_its_agents_at_haikus_rate_not_sonnets() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    // The mock counts tokens by whitespace: four in, two out.
    let factory = ApiFactory::new(
        Arc::new(MockProvider {
            reply: "two words".into(),
        }),
        64,
    )
    .with_model("claude-haiku-4-5");
    let agent = factory.make(&AgentKind::Api { system: None });
    let ctx = AgentContext {
        budget: crate::tools::budget::ToolBudget::solo(),
        agent: AgentId(0),
        task: TaskSpec {
            id: TaskId(0),
            title: "t".into(),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: vec![],
            prompt: "one two three four".into(),
            specialty: String::new(),
            expertise: String::new(),
        },
        deps: vec![],
        bus: bus.clone(),
    };
    assert!(agent.run(ctx).await.success);
    let mut cost = None;
    while let Ok(ev) = rx.try_recv() {
        if let HiveEvent::CostDelta { micros_usd, .. } = ev {
            cost = Some(micros_usd);
        }
    }
    assert_eq!(cost, Some(4 + 2 * 5), "haiku's rate");
    assert_ne!(
        cost,
        Some(cost_microusd("claude-sonnet-4-6", 4, 2)),
        "sonnet"
    );
}
