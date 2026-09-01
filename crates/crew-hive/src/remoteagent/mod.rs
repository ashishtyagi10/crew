//! `RemoteAgent`: an `Agent` that dispatches to a remote worker via `Transport`,
//! plus `RemoteFactory` to run a whole graph over one shared transport.
use crate::agent::{Agent, AgentContext, AgentFactory};
use crate::board::TaskResult;
use crate::bus::HiveEvent;
use crate::graph::AgentKind;
use crate::wire::{DepResult, Host, RemoteTask, ToolDecl, Transport};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// An agent that dispatches its task over a `Transport` to a remote worker.
pub struct RemoteAgent {
    transport: Arc<dyn Transport>,
    /// The tools the worker may ask crew to run. `None` is a worker that can only think.
    tools: Option<Arc<dyn crate::tools::Tools>>,
}

impl RemoteAgent {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            tools: None,
        }
    }

    /// The same agent with crew's tools behind it — the difference between a sidecar that can
    /// reach the world through crew's gate and one that can only return a string.
    pub fn with_tools(mut self, tools: Option<Arc<dyn crate::tools::Tools>>) -> Self {
        self.tools = tools;
        self
    }
}

impl Agent for RemoteAgent {
    fn run(&self, ctx: AgentContext) -> Pin<Box<dyn Future<Output = TaskResult> + Send>> {
        let transport = Arc::clone(&self.transport);
        let tools = self.tools.clone();
        Box::pin(async move {
            let decls: Vec<ToolDecl> = tools
                .as_ref()
                .map(|t| t.specs_for(&ctx.task.prompt))
                .unwrap_or_default()
                .into_iter()
                .map(|s| ToolDecl {
                    name: s.label(),
                    description: s.description,
                    input_schema: s.input_schema,
                })
                .collect();
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
                tools: decls,
                state: None,
            };
            // Streamed thinking reaches the pane as it happens, the same as a native agent's.
            let delta_bus = ctx.bus.clone();
            let delta_agent = ctx.agent.clone();
            let on_delta = move |s: &str| {
                delta_bus.publish(HiveEvent::OutputDelta {
                    agent: delta_agent.clone(),
                    text: s.to_string(),
                });
            };
            let host = Host {
                tools: tools.as_deref(),
                on_delta: &on_delta,
            };
            match transport.dispatch(rt, host).await {
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

/// Agent factory making [`RemoteAgent`]s that share one [`Transport`]. Hand the
/// scheduler a `RemoteFactory` to run an entire graph over a remote/sidecar
/// worker — the in-process [`crate::worker::LoopbackTransport`] makes this
/// testable without spawning anything.
pub struct RemoteFactory {
    transport: Arc<dyn Transport>,
    tools: Option<Arc<dyn crate::tools::Tools>>,
}

impl RemoteFactory {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            tools: None,
        }
    }

    /// Every agent this factory makes reaches crew's tools.
    pub fn with_tools(mut self, tools: Option<Arc<dyn crate::tools::Tools>>) -> Self {
        self.tools = tools;
        self
    }
}

impl AgentFactory for RemoteFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        Box::new(RemoteAgent::new(Arc::clone(&self.transport)).with_tools(self.tools.clone()))
    }
}
