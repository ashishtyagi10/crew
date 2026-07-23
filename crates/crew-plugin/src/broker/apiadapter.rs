//! API-backed agents: instead of shelling out to external CLIs, these drive
//! the relay by calling the LLM API in-process via crew-hive's [`Provider`].
//! There is no fixed roster any more — every [`ApiAdapter`] is either a
//! planner-invented specialist persisted to the project-local store (see
//! [`super::specialists`]) or a transient one-shot adapter built straight
//! from the discovered provider (see the `ask` module's one-shot asks). The
//! broker engine is synchronous, so each [`Adapter::call`] blocks on the async
//! provider with a small current-thread tokio runtime. The relay protocol
//! (`@next`/`@done`, peers, transcript) already arrives in the framed `body`
//! (see [`super::route::frame`]); the name and role only select the model + a
//! light system prompt.
use std::sync::Arc;
use std::time::Duration;

use crew_hive::{CompletionRequest, Provider};

use super::adapter::Adapter;

/// Output token ceiling per agent reply. Bounded so a runaway reply can't blow
/// the thread's cost; the relay favours concise hand-offs anyway.
const MAX_TOKENS: u32 = 2048;

/// An agent driven by an in-process LLM API call rather than an external CLI.
pub struct ApiAdapter {
    name: String,
    model: String,
    /// This agent's own capability hint. Held here rather than looked up by
    /// name: `Adapter::role`'s default consults `agents::role_for`, a static
    /// match over the known CLI names, which returns "" for an invented
    /// specialist — blanking the palette, peer list and roster badge.
    role: String,
    system: Option<String>,
    provider: Arc<dyn Provider>,
    /// Current-thread runtime to block the sync broker on the async provider.
    rt: tokio::runtime::Runtime,
}

impl ApiAdapter {
    /// Build an adapter named `name` calling `model`, with a `role` hint, an
    /// optional `system` prompt, backed by `provider`. Fails only if the
    /// tokio runtime can't start.
    pub fn new(
        name: impl Into<String>,
        model: impl Into<String>,
        role: impl Into<String>,
        system: Option<String>,
        provider: Arc<dyn Provider>,
    ) -> std::io::Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            name: name.into(),
            model: model.into(),
            role: role.into(),
            system,
            provider,
            rt,
        })
    }

    /// A planner-invented specialist: `name` is its `@`-handle (a slug),
    /// `role` its craft hint (possibly empty). The system prompt is derived
    /// here rather than stored, so a persisted specialist never pins stale
    /// prompt text.
    pub fn specialist(
        name: impl Into<String>,
        role: impl Into<String>,
        model: impl Into<String>,
        provider: Arc<dyn Provider>,
    ) -> std::io::Result<Self> {
        let (name, role) = (name.into(), role.into());
        let system = if role.is_empty() {
            format!(
                "You are the {name}. Do the work the task asks for, in your own \
                 specialty. Be concise."
            )
        } else {
            format!(
                "You are the {name}. Your specialty is {role}. Do the work the \
                 task asks for, from that expertise. Be concise."
            )
        };
        Self::new(name, model, role, Some(system), provider)
    }
}

impl Adapter for ApiAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn role(&self) -> &str {
        &self.role
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
                    cost_microusd: if c.cost_microusd > 0 {
                        c.cost_microusd
                    } else {
                        crew_hive::pricing::cost_microusd(
                            &self.model,
                            c.input_tokens,
                            c.output_tokens,
                        )
                    },
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
                    cost_microusd: if c.cost_microusd > 0 {
                        c.cost_microusd
                    } else {
                        crew_hive::pricing::cost_microusd(
                            &self.model,
                            c.input_tokens,
                            c.output_tokens,
                        )
                    },
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

/// Build one adapter per stored specialist on `provider`. `overrides` pins a
/// specific model per agent name (the `/model` construct). Adapters whose
/// runtime fails to start are skipped rather than aborting the roster.
///
/// There is no inbuilt roster any more: a fresh project has no specialists
/// until a run invents some. See the design doc.
pub fn specialist_agents(
    provider: Arc<dyn Provider>,
    model: &str,
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Adapter>> {
    super::specialists::load()
        .into_iter()
        .filter_map(|s| {
            let model = overrides
                .get(&s.name)
                .cloned()
                .unwrap_or_else(|| model.to_string());
            ApiAdapter::specialist(s.name, s.role, model, provider.clone())
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
        let a = ApiAdapter::new("planner", "m", "", None, mock("  do this\n@done  ")).unwrap();
        assert_eq!(
            a.call("task", Duration::from_secs(5)).unwrap(),
            "do this\n@done"
        );
    }

    #[test]
    fn a_specialist_reports_its_own_role() {
        let a = ApiAdapter::specialist("archivist", "records, retrieval", "m", mock("hi")).unwrap();
        assert_eq!(a.name(), "archivist");
        assert_eq!(a.role(), "records, retrieval");
    }

    #[test]
    fn a_specialists_system_prompt_carries_its_name_and_role() {
        let a = ApiAdapter::specialist("archivist", "records, retrieval", "m", mock("hi")).unwrap();
        let sys = a
            .system
            .clone()
            .expect("specialists always get a system prompt");
        assert!(sys.contains("archivist"), "got {sys}");
        assert!(sys.contains("records, retrieval"), "got {sys}");
    }

    #[test]
    fn a_roleless_specialist_still_gets_a_usable_prompt() {
        // expertise is allowed to be empty; the prompt must not read as
        // "Your specialty is ." in that case.
        let a = ApiAdapter::specialist("mystery", "", "m", mock("hi")).unwrap();
        let sys = a.system.clone().unwrap();
        assert!(sys.contains("mystery"), "got {sys}");
        assert!(!sys.contains("specialty is ."), "got {sys}");
    }

    #[test]
    fn ticked_call_reports_growing_char_estimates() {
        // MockProvider streams ~3 chunks; the estimator must report a
        // non-decreasing chars/4 sequence and the final text must match.
        let adapter = ApiAdapter::new(
            "planner",
            "m",
            "",
            None,
            mock("one two three four five six"),
        )
        .unwrap();
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
        let adapter = ApiAdapter::new("planner", "m", "", None, mock("文文文文 文文文文")).unwrap();
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
