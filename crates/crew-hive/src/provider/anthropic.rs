use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use super::wire::wire_error;
use super::{
    request_timeout, ChunkFn, Completion, CompletionRequest, Provider, ProviderError,
    ToolInvocation, Turn,
};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";
/// The beta header an OAuth bearer must travel with (endpoint-dependent in
/// theory; `/v1/messages` refuses the token without it).
const OAUTH_BETA: &str = "oauth-2025-04-20";

/// `<base>/v1/messages`, tolerant of a trailing slash or an already
/// `/v1`-suffixed base.
pub fn messages_url(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/messages")
    } else {
        format!("{base}/v1/messages")
    }
}

/// How a request proves who it is. An API key rides in `x-api-key` (the
/// form every crew install has always sent); an OAuth access token — the
/// short-lived `sk-ant-oat01-…` bearer the Anthropic CLI mints for a signed-in
/// Console profile — rides as `Authorization: Bearer` and MUST carry the
/// `oauth-2025-04-20` beta header, or `/v1/messages` refuses it.
#[derive(Clone)]
enum Auth {
    Key(String),
    OAuth(String),
}

/// Cloning is cheap: `reqwest::Client` is an `Arc` internally (shares one
/// connection pool) and the credential is a short `String`. Sharing one
/// provider between the planner and the worker factory relies on this.
#[derive(Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    auth: Auth,
    endpoint: String,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
    /// `thinking` blocks' text. Kept when a reply carries one; never asked
    /// for: requesting extended thinking alongside tool use means echoing
    /// the SIGNED blocks back verbatim on every later turn, which the
    /// `Turn` history has no slot for — out of scope here, and said so.
    #[serde(default)]
    thinking: String,
    // `tool_use` blocks. Defaulted rather than in a second struct so one
    // `content` array parses whatever mix of blocks a reply carries.
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    content: Vec<Block>,
    /// Read loosely (`served::anthropic_cache`): the cache fields may be
    /// absent or null, and a surprise there must not fail the reply.
    #[serde(default)]
    usage: Option<serde_json::Value>,
    /// The model that answered (`Completion::model`).
    #[serde(default)]
    model: Option<String>,
    /// `"max_tokens"` when the ceiling, not the model, ended the reply.
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_auth(Auth::Key(api_key))
    }

    /// A provider that authenticates with an OAuth access token (bearer +
    /// the OAuth beta header) instead of an API key.
    pub fn with_oauth(access_token: String) -> Self {
        Self::with_auth(Auth::OAuth(access_token))
    }

    fn with_auth(auth: Auth) -> Self {
        Self {
            client: super::io::client(request_timeout()),
            auth,
            endpoint: ENDPOINT.to_string(),
        }
    }

    /// Send requests to `<base>/v1/messages` instead of the live API — the
    /// `ANTHROPIC_BASE_URL` seam the official SDKs honour, and the way a
    /// test points this provider at a loopback stub.
    pub fn with_base_url(mut self, base: &str) -> Self {
        self.endpoint = messages_url(base);
        self
    }

    /// The endpoint requests go to.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// The auth headers one request carries — pure, so the two forms are
    /// table-testable without a socket. Never logged.
    pub fn auth_headers(&self) -> Vec<(&'static str, String)> {
        match &self.auth {
            Auth::Key(k) => vec![("x-api-key", k.clone())],
            Auth::OAuth(t) => vec![
                ("authorization", format!("Bearer {t}")),
                ("anthropic-beta", OAUTH_BETA.to_string()),
            ],
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        Self::from_key(std::env::var("ANTHROPIC_API_KEY").ok())
    }

    /// [`Self::from_env`]'s decision, with the value passed in.
    ///
    /// Split out so the rule can be TESTED. The test used to read the process
    /// environment and assert only when the key happened to be absent, which
    /// means it asserted nothing at all on any machine that had one — the
    /// machines most likely to be running it.
    pub fn from_key(key: Option<String>) -> Result<Self, ProviderError> {
        match key {
            Some(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey("ANTHROPIC_API_KEY")),
        }
    }

    pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
        let r: ApiResp =
            serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
        if r.kind == "error" || r.error.is_some() {
            return Err(ProviderError::Api(body.to_string()));
        }
        // EVERY text block, joined — not just the first. A reply that thinks,
        // calls a tool, then thinks again carries several, and taking only
        // `find` silently dropped the rest.
        let text = r
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        let thought = r
            .content
            .iter()
            .filter(|b| b.kind == "thinking")
            .map(|b| b.thinking.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let calls: Vec<ToolInvocation> = r
            .content
            .iter()
            .filter(|b| b.kind == "tool_use")
            .map(|b| ToolInvocation {
                id: b.id.clone(),
                name: b.name.clone(),
                input: b.input.clone().unwrap_or_else(|| serde_json::json!({})),
                bad_args: None,
            })
            .collect();
        let truncated = super::stopreason::anthropic(r.stop_reason.as_deref());
        let usage = r
            .usage
            .filter(|u| u.is_object())
            .ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
        let (cached_input_tokens, cache_write_tokens) = super::served::anthropic_cache(&usage);
        Ok(Completion {
            text,
            thought,
            model: r.model.unwrap_or_default().trim().to_string(),
            input_tokens: super::served::anthropic_input(&usage),
            cached_input_tokens,
            cache_write_tokens,
            output_tokens: super::served::count(&usage["output_tokens"]),
            cost_microusd: 0,
            calls,
            truncated,
        })
    }
}

/// The `messages` array for one request: the opening user prompt, then every
/// turn since.
///
/// Anthropic requires the assistant's `tool_use` blocks to be REPLAYED in the
/// history — a tool result whose `tool_use_id` has no matching call in a prior
/// assistant turn is rejected outright — so the conversation is rebuilt in
/// full on every request rather than sending only the newest exchange.
pub(crate) fn build_messages(req: &CompletionRequest) -> Vec<serde_json::Value> {
    let mut messages = vec![serde_json::json!({"role": "user", "content": req.prompt})];
    for turn in &req.turns {
        match turn {
            Turn::Assistant { text, calls } => {
                let mut content = Vec::new();
                if !text.is_empty() {
                    content.push(serde_json::json!({"type": "text", "text": text}));
                }
                for c in calls {
                    content.push(serde_json::json!({
                        "type": "tool_use",
                        "id": c.id,
                        "name": c.name,
                        "input": c.input,
                    }));
                }
                messages.push(serde_json::json!({"role": "assistant", "content": content}));
            }
            // Results go back as a USER turn — they are input to the model,
            // not something it said.
            Turn::ToolResults(results) => {
                let content: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": r.id,
                            "content": r.content,
                            "is_error": r.is_error,
                        })
                    })
                    .collect();
                messages.push(serde_json::json!({"role": "user", "content": content}));
            }
            Turn::User(text) => {
                messages.push(serde_json::json!({"role": "user", "content": text}));
            }
        }
    }
    messages
}

/// The tools array, or `None` when there are none. ABSENT, not empty: an
/// empty `tools: []` is a different request and some endpoints reject it.
pub(crate) fn build_tools(req: &CompletionRequest) -> Option<serde_json::Value> {
    (!req.tools.is_empty()).then(|| {
        req.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema,
                })
            })
            .collect()
    })
}

impl AnthropicProvider {
    /// One request's body; `stream` asks for server-sent events.
    pub(super) fn body(req: &CompletionRequest, stream: bool) -> serde_json::Value {
        Self::body_if(super::anthropiccache::enabled(), req, stream)
    }

    /// [`Self::body`] with the prompt-cache switch handed in rather than read
    /// from the environment (`anthropiccache`).
    pub(super) fn body_if(cache: bool, req: &CompletionRequest, stream: bool) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": req.model,
            "max_tokens": req.max_tokens,
            "messages": build_messages(req),
        });
        if let Some(sys) = &req.system {
            body["system"] = serde_json::json!(sys);
        }
        if let Some(tools) = build_tools(req) {
            body["tools"] = tools;
        }
        if super::anthropiccache::caches(cache, req) {
            body["cache_control"] = super::anthropiccache::field();
        }
        if stream {
            body["stream"] = serde_json::json!(true);
        }
        body
    }

    /// The request, sent — status left for the caller to read.
    async fn send(
        client: &reqwest::Client,
        endpoint: &str,
        headers: &[(&'static str, String)],
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, ProviderError> {
        let mut r = client.post(endpoint);
        for (k, v) in headers {
            r = r.header(*k, v);
        }
        r.header("anthropic-version", VERSION)
            .header("content-type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(wire_error(&e, endpoint)))
    }

    /// The request, sent again while it fails in a way that passes — a 429,
    /// a 5xx, Anthropic's 529 `overloaded_error` — after the wait the server
    /// asked for ([`super::retry`]). A success comes back unread; a failure
    /// that stands, as what its body says (`status::failure`). `attempt` is
    /// the caller's, so a stream that is retried after an in-stream error
    /// shares the one budget.
    async fn send_retrying(
        client: &reqwest::Client,
        endpoint: &str,
        headers: &[(&'static str, String)],
        body: &serde_json::Value,
        attempt: &mut u32,
    ) -> Result<reqwest::Response, ProviderError> {
        loop {
            let resp = Self::send(client, endpoint, headers, body).await?;
            if resp.status().is_success() {
                return Ok(resp);
            }
            let status = resp.status().as_u16();
            let hint = super::retry::retry_after(resp.headers());
            let text = resp.text().await.unwrap_or_default();
            match super::retry::again(status, hint, &text, attempt) {
                Some(wait) => tokio::time::sleep(wait).await,
                None => return Err(super::status::failure(status, endpoint, &text)),
            }
        }
    }
}

impl Provider for AnthropicProvider {
    fn supports_tools(&self) -> bool {
        true
    }

    /// Without [`Self::auth_headers`]: the key or bearer is for the call, and
    /// the socket opens without it.
    fn warm(&self) {
        super::warm::warm(&self.client, &self.endpoint);
    }

    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let client = self.client.clone();
        let headers = self.auth_headers();
        let endpoint = self.endpoint.clone();
        super::io::run(async move {
            let body = AnthropicProvider::body(&req, false);
            let resp =
                AnthropicProvider::send_retrying(&client, &endpoint, &headers, &body, &mut 0)
                    .await?;
            let status = resp.status().as_u16();
            let text = resp
                .text()
                .await
                .map_err(|e| ProviderError::Http(wire_error(&e, &endpoint)))?;
            let parse = AnthropicProvider::parse_response;
            super::status::settle(status, &endpoint, &text, parse).map(|c| c.served_by(&req.model))
        })
    }

    /// The reply as it is written (`anthropicsse`). A request carrying TOOLS
    /// is still made whole: a streamed `tool_use` arrives as `input_json_delta`
    /// fragments to reassemble, and the text-only reply is the one a person
    /// is watching type.
    fn complete_streaming(
        &self,
        req: CompletionRequest,
        on_chunk: ChunkFn,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        if !req.tools.is_empty() {
            return self.complete(req);
        }
        let client = self.client.clone();
        let headers = self.auth_headers();
        let endpoint = self.endpoint.clone();
        super::io::run(async move {
            use futures::StreamExt;
            let body = AnthropicProvider::body(&req, true);
            let mut attempt = 0;
            loop {
                let resp = AnthropicProvider::send_retrying(
                    &client,
                    &endpoint,
                    &headers,
                    &body,
                    &mut attempt,
                )
                .await?;
                let mut fold = super::anthropicsse::Fold::default();
                // Decoded where characters end, not where reads do: an em
                // dash split across two reads reached the pane as `��`.
                let mut utf8 = super::utf8carry::Utf8Carry::default();
                let mut stream = resp.bytes_stream();
                while let Some(bytes) = stream.next().await {
                    let bytes =
                        bytes.map_err(|e| ProviderError::Http(wire_error(&e, &endpoint)))?;
                    fold.feed(&utf8.push(&bytes), &on_chunk);
                }
                fold.feed(&utf8.finish(), &on_chunk);
                // Retried ONLY while nothing has reached the callback. A
                // refused request (`send_retrying`) has shown nothing, and a
                // busy host can also answer 200 and then send an `error`
                // event (`overloaded_error`) as its first word. Once a
                // fragment has been shown, a second try would type the
                // reply out again under it, so that failure stands.
                let unseen = fold.out.text.is_empty() && fold.out.thought.is_empty();
                let wait = match &fold.failed {
                    Some(e) if unseen => super::retry::again(200, None, e, &mut attempt),
                    _ => None,
                };
                match wait {
                    Some(wait) => tokio::time::sleep(wait).await,
                    None => return fold.finish(&on_chunk).map(|c| c.served_by(&req.model)),
                }
            }
        })
    }
}

#[cfg(test)]
#[path = "anthropicstream_tests.rs"]
mod stream_tests;

#[cfg(test)]
#[path = "anthropicretry_tests.rs"]
pub(super) mod retry_tests;
