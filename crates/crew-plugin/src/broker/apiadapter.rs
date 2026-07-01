//! Inbuilt agents: instead of shelling out to external CLIs, these drive the
//! relay by calling the LLM API in-process via crew-hive's [`Provider`]. The
//! broker engine is synchronous, so each [`Adapter::call`] blocks on the async
//! provider with a small current-thread tokio runtime. The relay protocol
//! (`@next`/`@done`, peers, transcript) already arrives in the framed `body`
//! (see [`super::route::frame`]); the role only selects the model + a light
//! system prompt.
use std::sync::Arc;
use std::time::Duration;

use crew_hive::{CompletionRequest, ModelTier, Provider};

use super::adapter::Adapter;

/// Output token ceiling per agent reply. Bounded so a runaway reply can't blow
/// the thread's cost; the relay favours concise hand-offs anyway.
const MAX_TOKENS: u32 = 2048;

/// An agent driven by an in-process LLM API call rather than an external CLI.
pub struct ApiAdapter {
    name: String,
    model: String,
    system: Option<String>,
    provider: Arc<dyn Provider>,
    /// Current-thread runtime to block the sync broker on the async provider.
    rt: tokio::runtime::Runtime,
}

impl ApiAdapter {
    /// Build an adapter named `name` calling `model`, with an optional `system`
    /// prompt, backed by `provider`. Fails only if the tokio runtime can't start.
    pub fn new(
        name: impl Into<String>,
        model: impl Into<String>,
        system: Option<String>,
        provider: Arc<dyn Provider>,
    ) -> std::io::Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            name: name.into(),
            model: model.into(),
            system,
            provider,
            rt,
        })
    }
}

impl Adapter for ApiAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    /// Inbuilt agents are only constructed when an API key is present, so they
    /// are always usable.
    fn probe(&self) -> bool {
        true
    }

    fn call(&self, body: &str, timeout: Duration) -> Result<String, String> {
        let req = CompletionRequest {
            model: self.model.clone(),
            system: self.system.clone(),
            prompt: body.to_string(),
            max_tokens: MAX_TOKENS,
        };
        let fut = self.provider.complete(req);
        match self
            .rt
            .block_on(async move { tokio::time::timeout(timeout, fut).await })
        {
            Ok(Ok(c)) => Ok(c.text.trim().to_string()),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err(format!("api call timed out after {timeout:?}")),
        }
    }
}

/// The inbuilt relay roster: a planner (Capable tier), a coder and a reviewer
/// (Standard tier), all backed by `provider`. `model_for` maps each role's tier
/// to a concrete model id, so the same roster works across providers (Anthropic
/// native ids vs. OpenRouter slugs). Adapters whose runtime fails to start are
/// skipped rather than aborting the whole roster.
pub fn inbuilt_agents(
    provider: Arc<dyn Provider>,
    model_for: impl Fn(ModelTier) -> String,
) -> Vec<Box<dyn Adapter>> {
    let specs: [(&str, ModelTier, &str); 3] = [
        (
            "planner",
            ModelTier::Capable,
            "You are the planner. Clarify the task, outline the approach and the \
             key steps, then hand off. Be concise.",
        ),
        (
            "coder",
            ModelTier::Standard,
            "You are the coder. Implement what the plan calls for with concrete, \
             correct code. Be concise.",
        ),
        (
            "reviewer",
            ModelTier::Standard,
            "You are the reviewer. Critique the work, catch bugs, gaps and risks. \
             Be concise.",
        ),
    ];
    specs
        .into_iter()
        .filter_map(|(name, tier, system)| {
            ApiAdapter::new(
                name,
                model_for(tier),
                Some(system.to_string()),
                provider.clone(),
            )
            .ok()
            .map(|a| Box::new(a) as Box<dyn Adapter>)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crew_hive::MockProvider;

    fn mock(reply: &str) -> Arc<dyn Provider> {
        Arc::new(MockProvider {
            reply: reply.to_string(),
        })
    }

    #[test]
    fn call_returns_the_providers_trimmed_reply() {
        let a = ApiAdapter::new("planner", "m", None, mock("  do this\n@done  ")).unwrap();
        assert_eq!(
            a.call("task", Duration::from_secs(5)).unwrap(),
            "do this\n@done"
        );
    }

    fn tier_model(t: ModelTier) -> String {
        t.model_id().to_string()
    }

    #[test]
    fn inbuilt_agents_are_planner_coder_reviewer() {
        let agents = inbuilt_agents(mock("ok"), tier_model);
        let names: Vec<&str> = agents.iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["planner", "coder", "reviewer"]);
    }

    #[test]
    fn inbuilt_agents_all_probe_usable() {
        assert!(inbuilt_agents(mock("ok"), tier_model)
            .iter()
            .all(|a| a.probe()));
    }
}
