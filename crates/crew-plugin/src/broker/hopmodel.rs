//! The model a relay hop's turn calls (`hopturn`): an API agent's own
//! provider, bounded as its dial was ([`Bounded`]), or a CLI agent spoken to
//! as one ([`Spoken`]). Either way each call's usage is kept for the hop.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crew_hive::{
    Chunk, ChunkFn, Completion, CompletionRequest, HiveEvent, Provider, ProviderError, Turn,
};

use super::{lock, Shared};
use crate::broker::adapter::{Adapter, HopStream, Usage};

type Call = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

/// An API agent's provider as its relay dial called it: bounded by the
/// broker's timeout, and its usage kept for the hop. The turn makes each
/// reply whole itself (`provider::cutoff`, `TurnSpec::call_timeout`).
pub(super) struct Bounded {
    pub(super) inner: Arc<dyn Provider>,
    pub(super) model: String,
    pub(super) timeout: Duration,
    pub(super) tally: Shared,
}

impl Bounded {
    fn call(&self, req: CompletionRequest, on_chunk: Option<ChunkFn>) -> Call {
        let (inner, tally) = (Arc::clone(&self.inner), Arc::clone(&self.tally));
        let (model, timeout) = (self.model.clone(), self.timeout);
        let sent = sent_chars(&req);
        Box::pin(async move {
            let call = match on_chunk {
                Some(f) => inner.complete_streaming(req, f),
                None => inner.complete(req),
            };
            let c = match tokio::time::timeout(timeout, call).await {
                Ok(got) => got?,
                Err(_) => {
                    return Err(ProviderError::Http(format!(
                    "{model}: api call timed out after {timeout:?} (raise CREW_BROKER_TIMEOUT_MS?)"
                )))
                }
            };
            let used = Usage::of(&model, &c);
            lock(&tally).landed(sent, &c, used);
            Ok(c)
        })
    }
}

impl Provider for Bounded {
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }

    fn complete(&self, req: CompletionRequest) -> Call {
        self.call(req, None)
    }

    fn warm(&self) {
        self.inner.warm();
    }

    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        self.call(req, Some(on_chunk))
    }
}

/// A CLI agent (`claude`, `codex`, `opencode`, a manifest agent) as a model
/// with no tool-use: a prompt in, its reply out, so the harness's text loop
/// runs the `@tool` lines it prints. Its own tools stay its own: what it
/// runs inside its turn (Claude Code's `Read`) goes straight to the pane's
/// tool lane, as it did, and never through crew's gate, which is not theirs.
///
/// Only the text loop calls it (`supports_tools` is false), whose requests
/// carry no turns, so the prompt is the whole request.
pub(super) struct Spoken {
    pub(super) agent: Arc<dyn Adapter>,
    pub(super) timeout: Duration,
    /// The hop's tool lane, for the agent's own tools.
    pub(super) lane: Arc<dyn Fn(HiveEvent) + Send + Sync>,
    pub(super) tally: Shared,
}

impl Spoken {
    fn call(&self, req: CompletionRequest, on_chunk: Option<ChunkFn>) -> Call {
        let (agent, tally) = (Arc::clone(&self.agent), Arc::clone(&self.tally));
        let (lane, timeout) = (Arc::clone(&self.lane), self.timeout);
        let stop = crate::broker::cancelscope::current();
        Box::pin(async move {
            let stream = streamed(on_chunk, lane);
            let prompt = req.prompt;
            // On a thread of its own, the task's stop scope entered there: an
            // agent that blocks on a runtime of its own would panic on this
            // one's. The CLI is waited for either way, as its dial was.
            let dialed = std::thread::scope(|s| {
                s.spawn(|| {
                    let _scope = stop.map(crate::broker::cancelscope::enter);
                    agent.call_with_usage_ticked(&prompt, timeout, &stream)
                })
                .join()
                .unwrap_or_else(|_| Err(format!("{}: the agent panicked", agent.name())))
            });
            let mut t = lock(&tally);
            match dialed {
                Ok((text, used)) => {
                    let c = Completion {
                        text,
                        input_tokens: used.input_tokens,
                        output_tokens: used.output_tokens,
                        cost_microusd: used.cost_microusd,
                        ..Default::default()
                    };
                    t.landed(prompt.len(), &c, used);
                    Ok(c)
                }
                // In the agent's own words for the error card: a provider
                // error would say them under its own prefix (`Tally::said`).
                // An `Api` body, so a context the agent's model refused is
                // read as one (`is_context_overflow`), and cut and retried.
                Err(why) => {
                    t.said = Some(why.clone());
                    Err(ProviderError::Api(why))
                }
            }
        })
    }
}

impl Provider for Spoken {
    fn complete(&self, req: CompletionRequest) -> Call {
        self.call(req, None)
    }

    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        self.call(req, Some(on_chunk))
    }
}

/// A hop stream into `on_chunk`, for an agent that streams: its text and
/// reasoning as the turn's chunks, which the pane counts for the hop's tick
/// (`Pane`), and its own tools onto `lane`. The empty reasoning that ends an
/// agent's call is not passed on: the turn flushes once, at its end.
fn streamed(on_chunk: Option<ChunkFn>, lane: Arc<dyn Fn(HiveEvent) + Send + Sync>) -> HopStream {
    let (text, thought) = (on_chunk.clone(), on_chunk);
    HopStream {
        on_tokens: Arc::new(|_| {}),
        on_text: Arc::new(move |s| {
            if let Some(f) = &text {
                f(Chunk::Text(s));
            }
        }),
        on_thought: Arc::new(move |s| {
            if let (Some(f), false) = (&thought, s.is_empty()) {
                f(Chunk::Thought(s));
            }
        }),
        on_tool: lane,
    }
}

/// Chars a request sends, its turns included, for the chars/4 estimate.
fn sent_chars(req: &CompletionRequest) -> usize {
    let turns: usize = req
        .turns
        .iter()
        .map(|t| match t {
            Turn::User(text) => text.len(),
            Turn::Assistant { text, calls } => {
                text.len()
                    + calls
                        .iter()
                        .map(|c| c.input.to_string().len())
                        .sum::<usize>()
            }
            Turn::ToolResults(results) => results.iter().map(|r| r.content.len()).sum(),
        })
        .sum();
    req.prompt.len() + turns
}
