//! `RemoteAgent`: an `Agent` that dispatches to a remote worker via `Transport`.
use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::wire::{DepResult, RemoteTask, Transport};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// An agent that dispatches its task over a `Transport` to a remote worker.
pub struct RemoteAgent {
    transport: Arc<dyn Transport>,
}

impl RemoteAgent {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self { transport }
    }
}

impl Agent for RemoteAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let transport = Arc::clone(&self.transport);
        Box::pin(async move {
            let rt = RemoteTask {
                agent: ctx.agent.0,
                task: ctx.task.id.0,
                prompt: ctx.task.prompt.clone(),
                model: ctx.task.model.model_id().to_string(),
                deps: ctx
                    .deps
                    .iter()
                    .map(|d| DepResult {
                        task: d.task.0,
                        output: d.output.clone(),
                        success: d.success,
                    })
                    .collect(),
            };
            match transport.dispatch(rt).await {
                Ok(reply) => {
                    ctx.bus.publish(HiveEvent::TokenDelta {
                        agent: ctx.agent.clone(),
                        input: reply.input_tokens,
                        output: reply.output_tokens,
                    });
                    ctx.bus.publish(HiveEvent::OutputChunk {
                        agent: ctx.agent.clone(),
                        text: reply.output.clone(),
                    });
                    TaskResult {
                        task: ctx.task.id,
                        output: reply.output,
                        success: reply.success,
                    }
                }
                Err(e) => {
                    ctx.bus.publish(HiveEvent::Failed {
                        agent: ctx.agent.clone(),
                        error: e.to_string(),
                    });
                    TaskResult {
                        task: ctx.task.id,
                        output: String::new(),
                        success: false,
                    }
                }
            }
        })
    }
}
