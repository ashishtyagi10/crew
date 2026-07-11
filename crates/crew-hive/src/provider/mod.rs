//! LLM provider abstraction: a `Provider` turns a prompt into a `Completion`.
//! Object-safe (boxed future, no async-trait) so the mock and the real
//! Anthropic client share one interface.
mod anthropic;
mod mock;
mod openai_http;
mod openrouter;
#[cfg(test)]
mod tests;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;
pub use openrouter::OpenRouterProvider;

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Per-attempt HTTP timeout: `CREW_HTTP_TIMEOUT_MS`, default 120s. Kept below
/// the broker's per-call cap (180s default) so when one endpoint stalls the
/// error names the transport and the model fallback chain still gets a turn,
/// instead of the outer cap killing the whole attempt with no diagnosis.
/// Non-streamed completions arrive in one final read, so this bounds the whole
/// silent generation wait — the observed worst case (2048 tokens on qwen-max)
/// is ~30s, leaving 4× headroom.
pub(crate) fn request_timeout() -> Duration {
    let ms = std::env::var("CREW_HTTP_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120_000);
    Duration::from_millis(ms)
}

/// The HTTP client every provider shares: bounded at each network layer so a
/// dead path fails fast with a reqwest error (which names the URL) rather than
/// hanging until the caller's outer timeout. Idle pooled sockets are dropped
/// after 30s — NAT boxes and VPNs silently kill longer-idle connections, and
/// reusing one of those corpses is exactly the "no response at all" hang;
/// keepalive probes cover the gap under 30s.
pub(crate) fn http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .tcp_keepalive(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        // Builder only fails on TLS/resolver misconfiguration; a plain client
        // (no timeouts) still works, so degrade rather than panic.
        .unwrap_or_else(|_| reqwest::Client::new())
}

#[derive(Clone, Debug)]
pub struct CompletionRequest {
    pub model: String,
    pub system: Option<String>,
    pub prompt: String,
    pub max_tokens: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug)]
pub enum ProviderError {
    Http(String),
    Decode(String),
    Api(String),
    MissingKey,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Http(s) => write!(f, "http error: {s}"),
            ProviderError::Decode(s) => write!(f, "decode error: {s}"),
            ProviderError::Api(s) => write!(f, "api error: {s}"),
            ProviderError::MissingKey => write!(f, "ANTHROPIC_API_KEY not set"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// Callback for a streamed completion: invoked with each text delta as it
/// arrives. `Arc` (not `Box`) so callers can clone it into an async block
/// while also holding a reference for bookkeeping.
pub type ChunkFn = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

pub trait Provider: Send + Sync {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

    /// Streamed completion: `on_chunk` receives each text delta as it
    /// arrives. Default ignores the callback and delegates to `complete`,
    /// so non-streaming providers work unchanged (and emit no ticks).
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let _ = on_chunk;
        self.complete(req)
    }
}
