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
        self.call_with_usage(body, timeout).map(|(t, _)| t)
    }

    /// API replies carry real usage: the provider's reported prompt/completion
    /// tokens (the prompt size is the agent's live context fill).
    fn call_with_usage(
        &self,
        body: &str,
        timeout: Duration,
    ) -> Result<(String, super::adapter::Usage), String> {
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
            Ok(Ok(c)) => Ok((
                c.text.trim().to_string(),
                super::adapter::Usage {
                    input_tokens: c.input_tokens,
                    output_tokens: c.output_tokens,
                },
            )),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err(format!(
                "{}: api call timed out after {timeout:?} (raise CREW_BROKER_TIMEOUT_MS?)",
                self.model
            )),
        }
    }

    /// Same call as `call_with_usage`, but streams the reply and reports a
    /// running chars/4 OUTPUT-token estimate to `on_tokens` as chunks arrive.
    fn call_with_usage_ticked(
        &self,
        body: &str,
        timeout: Duration,
        on_tokens: Arc<dyn Fn(u64) + Send + Sync>,
    ) -> Result<(String, super::adapter::Usage), String> {
        let req = CompletionRequest {
            model: self.model.clone(),
            system: self.system.clone(),
            prompt: body.to_string(),
            max_tokens: MAX_TOKENS,
        };
        let chars = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter = chars.clone();
        let on_chunk: crew_hive::ChunkFn = Arc::new(move |s: &str| {
            // Unicode chars, not bytes — byte counts over-report CJK ~3×
            // (same convention as the provider-side chars/4 estimators).
            let n = s.chars().count() as u64;
            let total = counter.fetch_add(n, std::sync::atomic::Ordering::SeqCst) + n;
            on_tokens(total / 4);
        });
        let fut = self.provider.complete_streaming(req, on_chunk);
        match self
            .rt
            .block_on(async move { tokio::time::timeout(timeout, fut).await })
        {
            Ok(Ok(c)) => Ok((
                c.text.trim().to_string(),
                super::adapter::Usage {
                    input_tokens: c.input_tokens,
                    output_tokens: c.output_tokens,
                },
            )),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err(format!(
                "{}: api call timed out after {timeout:?} (raise CREW_BROKER_TIMEOUT_MS?)",
                self.model
            )),
        }
    }
}

/// The inbuilt relay roster: a planner (Capable tier), a coder and a reviewer
/// (Standard tier), all backed by `provider`. `model_for` maps each role's tier
/// to a concrete model id, so the same roster works across providers (Anthropic
/// native ids vs. OpenRouter slugs); `overrides` pins a specific model per
/// agent name (the `/model` construct), letting different agents run different
/// models side by side. Adapters whose runtime fails to start are skipped
/// rather than aborting the whole roster.
pub fn inbuilt_agents(
    provider: Arc<dyn Provider>,
    model_for: impl Fn(ModelTier) -> String,
    overrides: &std::collections::HashMap<String, String>,
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
            let model = overrides
                .get(name)
                .cloned()
                .unwrap_or_else(|| model_for(tier));
            ApiAdapter::new(name, model, Some(system.to_string()), provider.clone())
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
        let agents = inbuilt_agents(mock("ok"), tier_model, &Default::default());
        let names: Vec<&str> = agents.iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["planner", "coder", "reviewer"]);
    }

    #[test]
    fn inbuilt_agents_all_probe_usable() {
        assert!(inbuilt_agents(mock("ok"), tier_model, &Default::default())
            .iter()
            .all(|a| a.probe()));
    }

    #[test]
    fn ticked_call_reports_growing_char_estimates() {
        // MockProvider streams ~3 chunks; the estimator must report a
        // non-decreasing chars/4 sequence and the final text must match.
        let adapter =
            ApiAdapter::new("planner", "m", None, mock("one two three four five six")).unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
        let sink = seen.clone();
        let on_tokens: Arc<dyn Fn(u64) + Send + Sync> = Arc::new(move |t| {
            sink.lock().unwrap().push(t);
        });
        let (text, _usage) = adapter
            .call_with_usage_ticked("task", Duration::from_secs(5), on_tokens)
            .unwrap();
        assert_eq!(text, "one two three four five six");
        let ticks = seen.lock().unwrap();
        assert!(ticks.len() >= 2, "mock streams >=2 chunks: {ticks:?}");
        assert!(
            ticks.windows(2).all(|w| w[0] <= w[1]),
            "estimates never shrink"
        );
        let total_chars = "one two three four five six".len() as u64;
        assert_eq!(
            *ticks.last().unwrap(),
            total_chars / 4,
            "final estimate = chars/4"
        );
    }

    #[test]
    fn ticked_estimates_count_chars_not_bytes() {
        // 8 CJK chars = 24 UTF-8 bytes: bytes/4 would report 6, chars/4
        // must report 2 (same convention as the provider-side estimators).
        let adapter = ApiAdapter::new("planner", "m", None, mock("文文文文 文文文文")).unwrap();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
        let sink = seen.clone();
        let on_tokens: Arc<dyn Fn(u64) + Send + Sync> = Arc::new(move |t| {
            sink.lock().unwrap().push(t);
        });
        let (text, _usage) = adapter
            .call_with_usage_ticked("task", Duration::from_secs(5), on_tokens)
            .unwrap();
        assert_eq!(text, "文文文文 文文文文");
        let ticks = seen.lock().unwrap();
        let total_chars = "文文文文 文文文文".chars().count() as u64; // 9
        assert_eq!(
            *ticks.last().unwrap(),
            total_chars / 4,
            "final estimate uses chars: {ticks:?}"
        );
    }
}
