//! Native API agent: calls a [`Provider`] in an async future, emitting
//! telemetry events as it goes. The default headless scale worker.

#[cfg(test)]
mod tests;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::graph::{AgentKind, ModelTier};
use crate::provider::{CompletionRequest, Provider};

// ---------------------------------------------------------------------------
// Cost table — micros-USD per token (input / output)
// ---------------------------------------------------------------------------

/// Approximate Anthropic pricing in micros-USD per token.
/// Cheap ≈ $1/$5 per 1M; Standard ≈ $3/$15; Capable ≈ $15/$75.
fn cost_micros(tier: ModelTier, input: u32, output: u32) -> u64 {
    let (in_rate, out_rate): (u64, u64) = match tier {
        ModelTier::Cheap => (1, 5),
        ModelTier::Standard => (3, 15),
        ModelTier::Capable => (15, 75),
    };
    in_rate * u64::from(input) + out_rate * u64::from(output)
}

// ---------------------------------------------------------------------------
// build_prompt
// ---------------------------------------------------------------------------

/// Build a prompt from the task's own prompt plus dependency outputs.
/// Pure and testable.
pub(crate) fn build_prompt(task_prompt: &str, deps: &[TaskResult]) -> String {
    if deps.is_empty() {
        return task_prompt.to_owned();
    }
    let mut out = task_prompt.to_owned();
    out.push_str("\n\nContext from dependencies:\n");
    for dep in deps {
        out.push_str(&format!("- {}\n", dep.output));
    }
    out
}

// ---------------------------------------------------------------------------
// ApiAgent
// ---------------------------------------------------------------------------

pub struct ApiAgent {
    provider: Arc<dyn Provider>,
    tier: ModelTier,
    max_tokens: u32,
}

impl ApiAgent {
    pub fn new(provider: Arc<dyn Provider>, tier: ModelTier, max_tokens: u32) -> Self {
        Self {
            provider,
            tier,
            max_tokens,
        }
    }
}

impl Agent for ApiAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let provider = Arc::clone(&self.provider);
        let tier = self.tier;
        let max_tokens = self.max_tokens;
        Box::pin(async move {
            let task_id = ctx.task.id;
            let agent_id = ctx.agent.clone();
            let prompt = build_prompt(&ctx.task.prompt, &ctx.deps);
            let system = match &ctx.task.agent {
                AgentKind::Api { system } => system.clone(),
                AgentKind::Pty { .. } => None,
            };
            let req = CompletionRequest {
                model: tier.model_id().to_owned(),
                system,
                prompt,
                max_tokens,
            };
            match provider.complete(req).await {
                Ok(completion) => {
                    ctx.bus.publish(HiveEvent::TokenDelta {
                        agent: agent_id.clone(),
                        input: completion.input_tokens,
                        output: completion.output_tokens,
                    });
                    ctx.bus.publish(HiveEvent::OutputChunk {
                        agent: agent_id.clone(),
                        text: completion.text.clone(),
                    });
                    ctx.bus.publish(HiveEvent::CostDelta {
                        agent: agent_id,
                        micros_usd: cost_micros(
                            tier,
                            completion.input_tokens,
                            completion.output_tokens,
                        ),
                    });
                    TaskResult {
                        task: task_id,
                        output: completion.text,
                        success: true,
                    }
                }
                Err(err) => {
                    ctx.bus.publish(HiveEvent::Failed {
                        agent: agent_id,
                        error: err.to_string(),
                    });
                    TaskResult {
                        task: task_id,
                        output: String::new(),
                        success: false,
                    }
                }
            }
        })
    }
}
