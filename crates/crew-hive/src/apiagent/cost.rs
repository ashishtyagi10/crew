//! What one model call cost, billed at the model that actually answered.
//!
//! WHY: the tier table below was the whole story when every task ran on its
//! tier's own Anthropic model. A factory pinned to one model id
//! (`ApiFactory::with_model` — every non-Anthropic provider, and Anthropic
//! once the swarm chose its tier) still left each task billing at the TIER's
//! rate, so a Haiku call was charged as Sonnet, and a Qwen call as Sonnet
//! too. The price list knows the model; the tier is only the fallback for an
//! id it has never heard of — and the tier's price is its own model's, from
//! the same list, so there are not two tables to drift apart (the old one
//! had Opus at three times its list price). A cost the provider reported
//! itself outranks both: it is not an estimate.
use crate::graph::ModelTier;
use crate::provider::Completion;

/// Micro-USD for `c`, answered by `model_id` on a task the planner put at
/// `tier`. One price list (`crate::pricing`) answers for both: the model
/// that answered when it is listed, else the tier's own default model — so
/// the estimate for an unlisted id is the tier's price, never zero.
pub(super) fn billed(model_id: &str, tier: ModelTier, c: &Completion) -> u64 {
    if c.cost_microusd > 0 {
        return c.cost_microusd;
    }
    let priced = match crate::pricing::rate(model_id) {
        Some(_) => model_id,
        None => tier.model_id(),
    };
    crate::pricing::cost_microusd(priced, c.input_tokens, c.output_tokens)
}

#[cfg(test)]
#[path = "cost_tests.rs"]
mod tests;
