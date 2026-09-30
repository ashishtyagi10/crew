//! [`ApiFactory`]: the swarm's maker of [`ApiAgent`]s, one per task, all
//! sharing what one run shares — the provider, the tool surface and its
//! approval gate, the project card, and what the user said mid-run.
//!
//! Split out of `apiagent/mod.rs`, which was over the line cap.

use std::sync::Arc;

use super::ApiAgent;
use crate::agent::Agent;
use crate::graph::AgentKind;
use crate::provider::Provider;
use crate::steers::Steers;
use crate::tools::Tools;

use crate::agent::AgentFactory;

/// Agent factory making native [`ApiAgent`]s that share one provider. Each
/// agent reads its model tier from its task at run time (see [`ApiAgent::attempt`]),
/// so the factory only needs the provider and the per-task output token cap.
pub struct ApiFactory {
    provider: Arc<dyn Provider>,
    max_tokens: u32,
    model: Option<String>,
    /// Shared by every agent the factory makes, so one MCP host and ONE
    /// approval gate serve the whole swarm. Handing each agent its own would
    /// mean a person approving the same irreversible tool once per agent.
    tools: Option<Arc<dyn Tools>>,
    preamble: Option<String>,
    /// The run's steers: one log, so every worker reads what any took.
    steers: Option<Steers>,
}

impl ApiFactory {
    pub fn new(provider: Arc<dyn Provider>, max_tokens: u32) -> Self {
        Self {
            provider,
            max_tokens,
            model: None,
            tools: None,
            preamble: None,
            steers: None,
        }
    }

    /// Let every agent this factory makes take what the user types mid-run
    /// (see [`crate::steers`]).
    pub fn with_steers(mut self, steers: Steers) -> Self {
        self.steers = Some(steers);
        self
    }

    /// End every agent's system prompt with `preamble` (see [`ApiAgent::with_preamble`]).
    pub fn with_preamble(mut self, preamble: impl Into<String>) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }

    /// Give every agent this factory makes the same tool surface.
    pub fn with_tools(mut self, tools: Arc<dyn Tools>) -> Self {
        self.tools = Some(tools);
        self
    }
}

impl AgentFactory for ApiFactory {
    fn make(&self, _kind: &AgentKind) -> Box<dyn Agent> {
        let mut agent = ApiAgent::new(Arc::clone(&self.provider), self.max_tokens);
        if let Some(m) = &self.model {
            agent = agent.with_model(m.clone());
        }
        if let Some(t) = &self.tools {
            agent = agent.with_tools(Arc::clone(t));
        }
        if let Some(p) = &self.preamble {
            agent = agent.with_preamble(p.clone());
        }
        if let Some(s) = &self.steers {
            agent = agent.with_steers(s.clone());
        }
        Box::new(agent)
    }
}
