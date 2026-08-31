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

/// One tool a model may call, with the JSON Schema for its arguments.
///
/// The schema is the point. crew's previous tool surface handed the model a
/// name and a 100-character description CLIP and asked it to write JSON by
/// guesswork; here the provider validates the shape and the model chooses on
/// the argument structure rather than on half a sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolDef {
    /// The WIRE name — `[a-zA-Z0-9_-]{1,64}`, which `server:tool` is not.
    /// See `crate::tools::wire_name`, which owns the encoding and the map back.
    pub name: String,
    pub description: String,
    /// JSON Schema object for the arguments. An empty object is legal and
    /// means "no arguments"; it must never be `null`, which some providers
    /// reject outright.
    pub input_schema: serde_json::Value,
}

/// A tool call the model made, as the provider reported it.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolInvocation {
    /// The provider's own id for this call. It is echoed back verbatim in the
    /// result and is how a provider pairs the two across a parallel batch —
    /// never reconstruct it, and never assume one call per turn.
    pub id: String,
    pub name: String,
    /// Arguments as the model produced them. Kept as `Value`, not `String`,
    /// so nothing re-parses provider-validated JSON.
    pub input: serde_json::Value,
}

/// The outcome of one [`ToolInvocation`], on its way back to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolOutcome {
    pub id: String,
    pub name: String,
    pub content: String,
    /// A refusal or failure. Providers have a flag for this; without it a
    /// model reads "ERROR: connection refused" as data it should use.
    pub is_error: bool,
}

/// One exchange after the opening user prompt.
#[derive(Clone, Debug, PartialEq)]
pub enum Turn {
    /// What the model said, and anything it asked to call.
    Assistant {
        text: String,
        calls: Vec<ToolInvocation>,
    },
    /// Results for the calls in the immediately preceding assistant turn.
    /// Providers require these to arrive together, in one turn, covering
    /// EVERY call — a missing result is a protocol error, not a partial answer.
    ToolResults(Vec<ToolOutcome>),
}

/// `Default` is derived so a new field can be added here without touching
/// every construction site again; the existing ones all spell
/// `..Default::default()`.
#[derive(Clone, Debug, Default)]
pub struct CompletionRequest {
    pub model: String,
    pub system: Option<String>,
    /// The opening user message.
    pub prompt: String,
    pub max_tokens: u32,
    /// Everything after `prompt`. Empty for a one-shot completion, which is
    /// what every caller but the tool loop wants.
    pub turns: Vec<Turn>,
    /// Tools the model may call. Empty = a plain completion, and providers
    /// must then send NO tools field at all: an empty array is not the same
    /// as absent, and some endpoints reject it.
    pub tools: Vec<ToolDef>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Completion {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Exact provider-reported cost in micro-USD (OpenRouter `usage.cost`);
    /// 0 when the provider doesn't report cost.
    pub cost_microusd: u64,
    /// Tool calls the model made in this reply. Empty on every provider that
    /// does not support tools, and on every reply that simply answered.
    pub calls: Vec<ToolInvocation>,
}

#[derive(Debug)]
pub enum ProviderError {
    Http(String),
    Decode(String),
    Api(String),
    /// No key for the named variable. It carries the name because
    /// `OpenRouterProvider` backs six providers through different variables —
    /// a fixed string here named `ANTHROPIC_API_KEY` for all of them.
    MissingKey(&'static str),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Http(s) => write!(f, "http error: {s}"),
            ProviderError::Decode(s) => write!(f, "decode error: {s}"),
            ProviderError::Api(s) => write!(f, "{}", api_message(s)),
            ProviderError::MissingKey(var) => write!(f, "{var} not set"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// A provider's API error, in a sentence rather than a JSON blob.
///
/// Providers answer failures with a JSON envelope, and it used to reach the
/// user verbatim — pasted into the chat and truncated mid-structure, so the
/// one useful sentence inside it arrived cut in half. The message field is
/// what a person needs; the envelope is what a debugger needs, and a
/// debugger can read the log.
///
/// An authentication failure additionally gets told what to DO. It is the
/// most common provider error there is (a typo'd, expired or revoked key)
/// and the only one the user can always fix.
pub fn api_message(body: &str) -> String {
    let msg = extract_message(body).unwrap_or_else(|| one_line(body));
    let low = msg.to_lowercase();
    let auth = [
        "api key",
        "unauthorized",
        "invalid_api_key",
        "authentication",
        "401",
    ]
    .iter()
    .any(|p| low.contains(p));
    if auth {
        return format!("provider rejected the key \u{2014} {msg} (/model replaces it)");
    }
    format!("api error: {msg}")
}

/// The `error.message` string from a provider's JSON envelope, if it has one.
/// Hand-rolled rather than a serde type: every provider nests it slightly
/// differently, and a parse failure here must degrade to showing the body,
/// never to hiding the error.
fn extract_message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .or_else(|| v.get("message"))?
        .as_str()?;
    (!msg.trim().is_empty()).then(|| one_line(msg))
}

/// Whitespace collapsed to single spaces and trimmed to something a status
/// line can hold — a multi-line body must not become a multi-line error.
fn one_line(s: &str) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.chars().take(200).collect()
}

/// Callback for a streamed completion: invoked with each text delta as it
/// arrives. `Arc` (not `Box`) so callers can clone it into an async block
/// while also holding a reference for bookkeeping.
pub type ChunkFn = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

pub trait Provider: Send + Sync {
    /// Whether this provider speaks native tool-use — `tools` on the request
    /// and structured calls on the reply.
    ///
    /// Defaults to FALSE so a provider that has not been taught the mapping
    /// cannot silently drop a `tools` array and return prose. The caller
    /// (`crate::apiagent`) reads this to choose between the native path and
    /// the `@tool` text convention, so a wrong `true` here is a swarm that
    /// advertises tools no model was ever shown.
    fn supports_tools(&self) -> bool {
        false
    }

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

    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        (**self).complete_streaming(req, on_chunk)
    }
}

#[cfg(test)]
mod api_message_tests {
    use super::*;

    #[test]
    fn a_rejected_key_says_so_and_says_what_to_do() {
        let body = r#"{"error":{"message":"Incorrect API key provided: sk-abc***.","type":"invalid_request_error"}}"#;
        let m = api_message(body);
        assert!(m.contains("rejected the key"), "{m}");
        assert!(m.contains("Incorrect API key provided"), "{m}");
        assert!(m.contains("/model"), "the way to fix it: {m}");
        assert!(!m.contains('{'), "no JSON envelope: {m}");
    }

    #[test]
    fn other_errors_keep_their_sentence_without_the_envelope() {
        let body = r#"{"error":{"message":"This model is overloaded","type":"server_error"}}"#;
        let m = api_message(body);
        assert_eq!(m, "api error: This model is overloaded");
    }

    /// A body that is not the expected shape must still show the error. The
    /// worst outcome here is hiding a failure behind a parse miss.
    #[test]
    fn an_unparseable_body_is_shown_not_swallowed() {
        let m = api_message("<html>502 Bad Gateway</html>");
        assert!(m.contains("502 Bad Gateway"), "{m}");
        assert!(!api_message("").is_empty());
    }

    /// Multi-line bodies collapse: an error is one line, however it arrived.
    #[test]
    fn a_sprawling_body_becomes_one_bounded_line() {
        let body = format!("{{\"error\":{{\"message\":\"{}\"}}}}", "x ".repeat(400));
        let m = api_message(&body);
        assert!(!m.contains('\n'), "{m}");
        assert!(m.chars().count() < 240, "len {}", m.chars().count());
    }
}
