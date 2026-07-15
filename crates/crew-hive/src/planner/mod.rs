//! Task planner: decomposes a goal string into a [`TaskGraph`].
//!
//! - [`StubPlanner`] — deterministic, no LLM; used by scheduler tests.
//! - [`LlmPlanner`] — prompts a [`Provider`] for a JSON task array.
//! - [`parse_plan`] — pure JSON → [`TaskGraph`] converter; unit-testable.

#[cfg(test)]
mod tests;

use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use crate::graph::{AgentKind, GraphError, ModelTier, TaskGraph, TaskId, TaskSpec};
use crate::provider::{CompletionRequest, Provider, ProviderError};

// ---------------------------------------------------------------------------
// PlanError
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum PlanError {
    Provider(ProviderError),
    Parse(String),
    Graph(GraphError),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Provider(e) => write!(f, "provider error: {e}"),
            PlanError::Parse(s) => write!(f, "parse error: {s}"),
            PlanError::Graph(e) => write!(f, "graph error: {e}"),
        }
    }
}

impl std::error::Error for PlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PlanError::Provider(e) => Some(e),
            PlanError::Graph(e) => Some(e),
            PlanError::Parse(_) => None,
        }
    }
}

impl From<ProviderError> for PlanError {
    fn from(e: ProviderError) -> Self {
        PlanError::Provider(e)
    }
}

impl From<GraphError> for PlanError {
    fn from(e: GraphError) -> Self {
        PlanError::Graph(e)
    }
}

// ---------------------------------------------------------------------------
// Planner trait
// ---------------------------------------------------------------------------

pub trait Planner: Send + Sync {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>>;
}

// ---------------------------------------------------------------------------
// StubPlanner
// ---------------------------------------------------------------------------

/// Deterministic planner for tests: builds `fanout` leaf tasks plus one merge
/// task that depends on all of them. No LLM required.
pub struct StubPlanner {
    pub fanout: usize,
}

impl Planner for StubPlanner {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        let fanout = self.fanout;
        let goal = goal.to_owned();
        Box::pin(async move {
            let mut tasks: Vec<TaskSpec> = (0..fanout)
                .map(|i| TaskSpec {
                    id: TaskId(i as u64),
                    title: format!("leaf-{i}"),
                    agent: AgentKind::Api { system: None },
                    model: ModelTier::Standard,
                    deps: vec![],
                    prompt: goal.clone(),
                    specialty: format!("leaf-{i}"),
                    expertise: String::new(),
                })
                .collect();
            let merge = TaskSpec {
                id: TaskId(fanout as u64),
                title: "merge".into(),
                agent: AgentKind::Api { system: None },
                model: ModelTier::Standard,
                deps: (0..fanout).map(|i| TaskId(i as u64)).collect(),
                prompt: goal,
                specialty: "merge".into(),
                expertise: String::new(),
            };
            tasks.push(merge);
            Ok(TaskGraph::new(tasks)?)
        })
    }
}

// ---------------------------------------------------------------------------
// LlmPlanner
// ---------------------------------------------------------------------------

/// The planner's system prompt. Every clause here was earned against an
/// observed failure on a real provider (see the design doc's Prompt spike):
/// the agent-noun rule fixes bare topic words ("security"); the
/// anti-anchoring clause exists because naming flavourful examples turned
/// them into a word bank (one revision assigned "epidemiologist" to packing a
/// suitcase). There is deliberately no character limit here — the model
/// ignored one — so length is enforced in `agentname::slug` instead.
const PLANNER_SYSTEM: &str = "\
You are a task planner. Decompose the user's goal into a JSON array of tasks. \
Each task is an object with integer `id` (0-based), short `title`, a `prompt` \
describing the work, `deps` (array of task ids that must finish first), \
`specialty`, and `expertise`.\n\
\n\
`specialty` names the specialist the task needs. Rules:\n\
- At most TWO words, joined by a hyphen. Never three.\n\
- It must be a person, not a subject: an agent noun. Write \"security-auditor\", \
not \"security\"; \"risk-assessor\", not \"risk\".\n\
- Name them for their craft, not for the task: a task titled \"Gather Project \
Details\" needs an \"archivist\", not a \"gatherer\".\n\
- Use the word a real practitioner of that work would call themselves. Do not \
borrow vocabulary from these instructions — an example word that does not \
genuinely fit the goal at hand is worse than a plain one.\n\
\n\
`expertise` is a short comma-separated phrase naming that specialist's craft, \
e.g. \"records, retrieval, provenance\".\n\
\n\
Return ONLY the JSON array, no prose.";

/// LLM-backed planner: prompts `provider` and parses the JSON response.
pub struct LlmPlanner<P: Provider> {
    pub provider: P,
    pub tier: ModelTier,
    /// When set, overrides `tier.model_id()` in the planning request —
    /// required for non-Anthropic providers (DashScope/OpenRouter), whose
    /// model ids the tier table doesn't know.
    pub model: Option<String>,
}

impl<P: Provider> LlmPlanner<P> {
    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }
}

impl<P: Provider> Planner for LlmPlanner<P> {
    fn plan(
        &self,
        goal: &str,
    ) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>> {
        let req = CompletionRequest {
            model: self
                .model
                .clone()
                .unwrap_or_else(|| self.tier.model_id().to_owned()),
            system: Some(PLANNER_SYSTEM.to_owned()),
            prompt: goal.to_owned(),
            max_tokens: 2048,
        };
        let fut = self.provider.complete(req);
        Box::pin(async move {
            let completion = fut.await?;
            parse_plan(&completion.text)
        })
    }
}

// ---------------------------------------------------------------------------
// parse_plan
// ---------------------------------------------------------------------------

/// The shape we accept from model output. Deliberately has **no** `agent`,
/// `command`, or `args` field: the model describes *what* work to do and *who*
/// should do it, never *how* to execute it. serde ignores any such extra keys,
/// so an attacker-influenced completion cannot smuggle one in. See the
/// security note below.
#[derive(Deserialize)]
struct PlanNode {
    id: u64,
    title: String,
    prompt: String,
    deps: Vec<u64>,
    #[serde(default)]
    specialty: Option<String>,
    #[serde(default)]
    expertise: Option<String>,
}

/// Convert a model-produced JSON task array into a [`TaskGraph`].
///
/// SECURITY INVARIANT: the JSON here is untrusted (LLM output, ultimately
/// influenced by the goal and any tool/context content). Every task it yields
/// is forced to [`AgentKind::Api`] — model output can never select a
/// process-executing [`AgentKind::Pty`] agent. This is the trust boundary that
/// keeps a future Pty executor from becoming a command-injection sink; the
/// `debug_assert!` and `parse_plan_*` tests fail loudly if it ever regresses.
///
/// `specialty` and `expertise` are model-authored but inert: a display label,
/// an `@`-handle, and a role hint fed into the specialist's system prompt.
/// Neither selects an executor, so neither can reach that sink. `specialty` is
/// still slugged, because it becomes an addressable handle and the `@`
/// tokenizers assume `^[a-z0-9-]+$` without enforcing it.
pub(crate) fn parse_plan(json: &str) -> Result<TaskGraph, PlanError> {
    let nodes: Vec<PlanNode> =
        serde_json::from_str(json).map_err(|e| PlanError::Parse(e.to_string()))?;
    let tasks: Vec<TaskSpec> = nodes
        .into_iter()
        .map(|n| TaskSpec {
            id: TaskId(n.id),
            title: n.title,
            agent: AgentKind::Api { system: None },
            model: ModelTier::Standard,
            deps: n.deps.into_iter().map(TaskId).collect(),
            prompt: n.prompt,
            specialty: crate::agentname::slug_or(n.specialty.as_deref().unwrap_or(""), n.id),
            expertise: crate::agentname::role_clamp(n.expertise.as_deref().unwrap_or("")),
        })
        .collect();
    debug_assert!(
        !tasks.iter().any(|t| t.agent.is_pty()),
        "parse_plan must never yield a process-executing Pty task from model output",
    );
    debug_assert!(
        tasks
            .iter()
            .all(|t| crate::agentname::slug(&t.specialty).as_deref() == Some(t.specialty.as_str())),
        "parse_plan must never yield a specialty that isn't a stable slug",
    );
    Ok(TaskGraph::new(tasks)?)
}
