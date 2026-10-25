//! LLM provider abstraction: a `Provider` turns a prompt into a `Completion`.
//! Object-safe (boxed future, no async-trait) so the mock and the real
//! Anthropic client share one interface.
mod anthropic;
mod anthropiccache;
mod anthropicsse;
mod claudecli;
#[cfg(test)]
mod claudecli_tests;
pub mod claudestream;
mod completion;
pub mod cutoff;
mod error;
mod fellback;
mod hint;
mod io;
mod mock;
mod openai_http;
mod openrouter;
mod overflow;
#[cfg(test)]
mod reply_tests;
mod retry;
mod served;
mod ssecalls;
mod sseframe;
mod status;
mod stopreason;
#[cfg(test)]
mod tests;
mod thinking;
mod thinktags;
mod tokencap;
mod toolargs;
#[cfg(test)]
mod toolwire_tests;
mod utf8carry;
mod warm;
mod wire;

pub use anthropic::AnthropicProvider;
pub use claudecli::ClaudeCliProvider;
pub use completion::{Completion, CompletionRequest, ToolDef, ToolInvocation, ToolOutcome, Turn};
pub use error::{api_message, ProviderError};
use error::{extract_message, one_line};
pub use mock::MockProvider;
pub use openrouter::OpenRouterProvider;
pub use overflow::says_context_overflow;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Per-attempt HTTP SILENCE budget: `CREW_HTTP_TIMEOUT_MS`, default 120s.
/// How long the endpoint may say NOTHING — the wait for the first byte, and
/// each gap between two frames of a streamed body. Kept below the broker's
/// per-call cap (180s default) so when one endpoint stalls the error names
/// the transport and the model fallback chain still gets a turn, instead of
/// the outer cap killing the whole attempt with no diagnosis.
///
/// It is NOT a budget for the whole call, and used to be: a total deadline
/// covers the body read too, so a stream still arriving at 120s — eight fan
/// agents writing a long answer each — died mid-sentence, every one of them
/// reporting reqwest's kind-only `error decoding response body`. A healthy
/// stream is never silent; a dead one is silent immediately. Non-streamed
/// completions arrive in one final read, so this still bounds their whole
/// silent generation wait (worst case observed: ~30s for 2048 tokens on
/// qwen-max, 4× headroom).
pub(crate) fn request_timeout() -> Duration {
    let ms = std::env::var("CREW_HTTP_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120_000);
    Duration::from_millis(ms)
}

/// The HTTP client every provider shares: bounded at each network layer so a
/// dead path fails fast with a reqwest error rather than hanging until the
/// caller's outer timeout, and so a LIVE path is never cut off (`timeout` is
/// `read_timeout`, a per-read silence bound that every arriving frame resets
/// — see [`request_timeout`] — not a deadline on the whole call). Idle pooled
/// sockets are dropped after 30s — NAT boxes and VPNs silently kill
/// longer-idle connections, and reusing one of those corpses is exactly the
/// "no response at all" hang; keepalive probes cover the gap under 30s.
pub(crate) fn http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(timeout)
        .tcp_keepalive(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        // Builder only fails on TLS/resolver misconfiguration; a plain client
        // (no timeouts) still works, so degrade rather than panic.
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// One streamed fragment: reply text, or a piece of the model's reasoning.
///
/// Two kinds on one callback rather than two callbacks: every implementer
/// already threads one `on_chunk`, and a provider that separates the two
/// (DashScope's `reasoning_content`, OpenRouter's `reasoning`, a `<think>`
/// tag in the text) can only tell the caller which is which at the moment
/// the fragment arrives — a second channel would have to be kept in step
/// with the first by every one of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chunk<'a> {
    Text(&'a str),
    Thought(&'a str),
}

/// Callback for a streamed completion: invoked with each [`Chunk`] as it
/// arrives. `Arc` (not `Box`) so callers can clone it into an async block
/// while also holding a reference for bookkeeping.
pub type ChunkFn = std::sync::Arc<dyn Fn(Chunk<'_>) + Send + Sync>;

pub trait Provider: Send + Sync {
    /// Whether this provider speaks native tool-use — `tools` on the request
    /// and structured calls on the reply.
    ///
    /// Defaults to FALSE so a provider that has not been taught the mapping
    /// cannot silently drop a `tools` array and return prose. The caller
    /// (`crate::harness`) reads this to choose between the native path and
    /// the `@tool` text convention, so a wrong `true` here is a swarm that
    /// advertises tools no model was ever shown.
    fn supports_tools(&self) -> bool {
        false
    }

    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

    /// Open the connection the next call will use, and return at once — the
    /// user started typing, and the handshake (~0.5 s measured) is better
    /// paid while they do than when they press Enter (see `warm.rs`).
    /// Default: nothing, for a provider with no socket of its own to open
    /// (the CLI, the mock).
    fn warm(&self) {}

    /// Streamed completion: `on_chunk` receives each text (or reasoning)
    /// delta as it arrives. Default ignores the callback and delegates to
    /// `complete`, so non-streaming providers work unchanged (and emit no
    /// ticks); their reasoning, if any, still lands in `Completion::thought`.
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let _ = on_chunk;
        self.complete(req)
    }
}

/// `Arc<dyn Provider>` (and any `Arc<P>`) is itself a Provider, so callers
/// that hold a dynamically-discovered provider (the broker) can feed it to
/// generic consumers like `LlmPlanner<P: Provider>` without re-wrapping.
impl<P: Provider + ?Sized> Provider for std::sync::Arc<P> {
    fn supports_tools(&self) -> bool {
        (**self).supports_tools()
    }

    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        (**self).complete(req)
    }

    fn warm(&self) {
        (**self).warm()
    }

    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        (**self).complete_streaming(req, on_chunk)
    }
}
