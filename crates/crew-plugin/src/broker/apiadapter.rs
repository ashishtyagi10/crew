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

use super::adapter::{Adapter, HopStream};

/// Output token ceiling per agent reply. 2048 was too few for real work — a
/// long answer, a `sys:write_file` of a whole file, a `sys:edit` with a big
/// `new` string — which stopped mid-sentence or mid-JSON. Providers bill the
/// output actually generated, not the ceiling, so a concise hand-off costs
/// what it did; the ceiling only bounds a runaway, and a reply that still
/// reaches it is continued once (`cutoff`). 4096, not 8192: some endpoints
/// (NVIDIA NIM models among them) reject a request whose ceiling is over the
/// model's own output limit, and a rejected request answers nothing at all.
const MAX_TOKENS: u32 = 4096;

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
    /// prompt text. Its opening line is the hive's own (`persona::identity`),
    /// the same one a swarm worker gets, so the two paths introduce the same
    /// specialist the same way.
    ///
    /// Who it is, and nothing about how long to answer or what to do: every
    /// prompt that reaches it says that itself (the relay frame's HOW TO
    /// REPLY, the fan's "directly and concisely", a review's or a commit
    /// message's format). A "Be concise." here met the relay's "answer in
    /// full" in the same call, and a model told both hedges between them.
    pub fn specialist(
        name: impl Into<String>,
        role: impl Into<String>,
        model: impl Into<String>,
        provider: Arc<dyn Provider>,
    ) -> std::io::Result<Self> {
        let (name, role) = (name.into(), role.into());
        let who = crew_hive::planner::persona::identity(&name, &role);
        let system = match role.is_empty() {
            true => who,
            false => format!("{who} Work from that expertise."),
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

    fn introduced(&self) -> bool {
        self.system.is_some()
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
            ..Default::default()
        };
        let fut = super::cutoff::whole(Arc::clone(&self.provider), req, None, timeout);
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
                        crew_hive::pricing::estimate(&self.model, &c).unwrap_or(0)
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

    /// Same call as `call_with_usage`, but streams the reply: reports a
    /// running chars/4 OUTPUT-token estimate, each raw text fragment AND each
    /// fragment of reasoning, through `stream`, as chunks arrive. A provider
    /// that hands its reasoning over whole (never streamed) has it forwarded
    /// once, after the fact; either way the hop ends with the empty flush
    /// `HopStream::on_thought` documents, so no tail is lost in the gate.
    fn call_with_usage_ticked(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
    ) -> Result<(String, super::adapter::Usage), String> {
        let req = CompletionRequest {
            model: self.model.clone(),
            system: self.system.clone(),
            prompt: body.to_string(),
            max_tokens: MAX_TOKENS,
            ..Default::default()
        };
        let chars = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let counter = chars.clone();
        let on_tokens = Arc::clone(&stream.on_tokens);
        let on_text = Arc::clone(&stream.on_text);
        let on_thought = Arc::clone(&stream.on_thought);
        let streamed_thought = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen_thought = streamed_thought.clone();
        let on_chunk: crew_hive::ChunkFn = Arc::new(move |c: crew_hive::Chunk<'_>| {
            let s = match c {
                crew_hive::Chunk::Text(s) => s,
                crew_hive::Chunk::Thought(s) => {
                    seen_thought.store(true, std::sync::atomic::Ordering::SeqCst);
                    if !s.is_empty() {
                        on_thought(s);
                    }
                    return;
                }
            };
            // Text first: it is what the user sees, and the token estimate
            // must never delay it.
            on_text(s);
            // Unicode chars, not bytes — byte counts over-report CJK ~3×
            // (same convention as the provider-side chars/4 estimators).
            let n = s.chars().count() as u64;
            let total = counter.fetch_add(n, std::sync::atomic::Ordering::SeqCst) + n;
            on_tokens(total / 4);
        });
        let fut = super::cutoff::whole(Arc::clone(&self.provider), req, Some(on_chunk), timeout);
        let outcome = self
            .rt
            .block_on(async move { tokio::time::timeout(timeout, fut).await });
        if let Ok(Ok(c)) = &outcome {
            if !c.thought.is_empty() && !streamed_thought.load(std::sync::atomic::Ordering::SeqCst)
            {
                (stream.on_thought)(&c.thought);
            }
        }
        (stream.on_thought)(""); // the hop is over: flush the gate's tail
        match outcome {
            Ok(Ok(c)) => Ok((
                c.text.trim().to_string(),
                super::adapter::Usage {
                    input_tokens: c.input_tokens,
                    output_tokens: c.output_tokens,
                    cost_microusd: if c.cost_microusd > 0 {
                        c.cost_microusd
                    } else {
                        crew_hive::pricing::estimate(&self.model, &c).unwrap_or(0)
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
#[path = "apiadapter_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "promptvoice_tests.rs"]
mod voice_tests;
