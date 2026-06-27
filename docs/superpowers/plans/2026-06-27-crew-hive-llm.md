# crew-hive LLM Provider + Planner + Native API Agent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Give `crew-hive` a real brain and real workers: an LLM `Provider` abstraction (with a headless mock + a `reqwest`-based Anthropic implementation), an LLM-backed `Planner` that decomposes a goal into a `TaskGraph`, and a **native API agent** (`Agent` impl that calls an LLM directly — no PTY/subprocess). Everything is unit-testable with a mock; only the live Anthropic path needs an API key.

**Architecture:** Three modules. `provider` defines `trait Provider` (object-safe, boxed-future — no `async-trait`), `CompletionRequest`/`Completion`, `ModelTier::model_id()` (cost tiering: Cheap→haiku, Standard→sonnet, Capable→opus), a `MockProvider` for tests, and an `AnthropicProvider` that POSTs to `/v1/messages` via `reqwest`. `planner` defines `trait Planner` + a deterministic `StubPlanner` + an `LlmPlanner` that prompts a provider for a JSON task graph and parses it into a `TaskGraph`. `apiagent` defines `ApiAgent` — an `Agent` (from the scheduler plan) that assembles a prompt from its task + gathered dep results, calls the provider, emits token/cost/output `HiveEvent`s, and returns a `TaskResult`. Native API agents are the swarm's default scale worker (futures, not processes).

**Tech Stack:** Rust, `reqwest` (json + rustls-tls — existing workspace dep), `serde`/`serde_json`, `tokio`, `cargo test` + `tokio::test`. The live Anthropic test is gated on `ANTHROPIC_API_KEY` and `#[ignore]` by default.

## Global Constraints

- Hard **200-line maximum per `.rs` file**, total. Split into submodules before crossing it.
- **No new dependencies** — only existing `[workspace.dependencies]`: `reqwest`, `serde`, `serde_json`, `tokio`, `futures`. Add `reqwest = { workspace = true }` and `futures = { workspace = true }` to crew-hive's Cargo.toml (both already in `[workspace.dependencies]`).
- **No `async-trait`** — object-safe traits return `Pin<Box<dyn Future + Send>>` (same pattern as the existing `Agent` trait).
- Boundary types (`CompletionRequest`, `Completion`, errors) derive serde where they cross a boundary.
- Dead code removed, not suppressed; `#[cfg(test)]` gating allowed.
- **Default Anthropic model per tier:** Cheap=`claude-haiku-4-5`, Standard=`claude-sonnet-4-6`, Capable=`claude-opus-4-8`. Use these exact IDs.
- **Anthropic request:** `POST https://api.anthropic.com/v1/messages`, headers `x-api-key`, `anthropic-version: 2023-06-01`, `content-type: application/json`; body `{model, max_tokens, system?, messages:[{role:"user",content:<prompt>}]}`. Response: `content[0].text`, `usage.input_tokens`, `usage.output_tokens`, `stop_reason`. (Non-streaming for v1; streaming is a later enhancement.)
- Consumes prior plans: `crate::graph::{TaskGraph, TaskSpec, TaskId, AgentKind, ModelTier}`, `crate::bus::{AgentId, EventBus, HiveEvent}`, `crate::board::TaskResult`, `crate::agent::{Agent, AgentContext}`.

---

### Task 1: Provider abstraction + mock + model tiers

**Files:**
- Create: `crates/crew-hive/src/provider/mod.rs`
- Create: `crates/crew-hive/src/provider/mock.rs`
- Create: `crates/crew-hive/src/provider/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod provider;`)
- Modify: `crates/crew-hive/Cargo.toml` (add `reqwest`, `futures` workspace deps)
- Modify: `crates/crew-hive/src/graph/spec.rs` (add `impl ModelTier { pub fn model_id(&self) -> &'static str }`)

**Interfaces:**
- Produces (in `crate::provider`):
  - `pub struct CompletionRequest { pub model: String, pub system: Option<String>, pub prompt: String, pub max_tokens: u32 }` — `Clone, Debug`.
  - `pub struct Completion { pub text: String, pub input_tokens: u32, pub output_tokens: u32 }` — `Clone, Debug, PartialEq`.
  - `pub enum ProviderError { Http(String), Decode(String), Api(String), MissingKey }` — `Debug`, `Display`, `Error`.
  - `pub trait Provider: Send + Sync { fn complete(&self, req: CompletionRequest) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>; }`
  - `pub struct MockProvider { pub reply: String }` — `Provider` impl returning `Completion { text: reply.clone(), input_tokens: prompt.split_whitespace().count() as u32, output_tokens: reply.split_whitespace().count() as u32 }`.
- Produces (in `crate::graph`): `impl ModelTier { pub fn model_id(&self) -> &'static str }` → Cheap→`"claude-haiku-4-5"`, Standard→`"claude-sonnet-4-6"`, Capable→`"claude-opus-4-8"`.

- [ ] **Step 1: Add deps + the model_id mapping (with its test)**

In `crates/crew-hive/Cargo.toml` under `[dependencies]` add:
```toml
reqwest = { workspace = true }
futures = { workspace = true }
```
In `crates/crew-hive/src/graph/spec.rs`, add the impl and a unit test in that file's test module (create one if absent):
```rust
impl ModelTier {
    /// The default Anthropic model id for this cost tier.
    pub fn model_id(&self) -> &'static str {
        match self {
            ModelTier::Cheap => "claude-haiku-4-5",
            ModelTier::Standard => "claude-sonnet-4-6",
            ModelTier::Capable => "claude-opus-4-8",
        }
    }
}
```
Test (RED first): `assert_eq!(ModelTier::Capable.model_id(), "claude-opus-4-8");` and one per variant. Run `cargo test -p crew-hive model_id` → fail, then pass.

- [ ] **Step 2: Write the failing provider tests**

Create `crates/crew-hive/src/provider/tests.rs`:
```rust
use super::*;

#[tokio::test]
async fn mock_provider_echoes_reply_and_counts() {
    let p = MockProvider { reply: "hello there".into() };
    let c = p
        .complete(CompletionRequest {
            model: "m".into(),
            system: None,
            prompt: "one two three".into(),
            max_tokens: 100,
        })
        .await
        .unwrap();
    assert_eq!(c.text, "hello there");
    assert_eq!(c.input_tokens, 3);
    assert_eq!(c.output_tokens, 2);
}

#[test]
fn provider_is_object_safe() {
    let _p: Box<dyn Provider> = Box::new(MockProvider { reply: "x".into() });
}
```

- [ ] **Step 3: Run to verify fail**

Run: `cargo test -p crew-hive provider::`
Expected: FAIL — `provider` not defined.

- [ ] **Step 4: Implement the provider module + mock**

Create `crates/crew-hive/src/provider/mod.rs` with `CompletionRequest`, `Completion`, `ProviderError` (Display+Error), and the `Provider` trait (boxed-future). Declare `mod mock; mod anthropic; #[cfg(test)] mod tests;` and re-export `MockProvider` and `AnthropicProvider`. (The `anthropic` submodule is written in Task 2; for this task create it as a stub file with just the struct + a `todo!()`-free minimal `Provider` impl OR defer its `mod` declaration to Task 2. Cleaner: declare only `mod mock;` here in Task 1 and add `mod anthropic;` in Task 2.)

```rust
//! LLM provider abstraction: a `Provider` turns a prompt into a `Completion`.
//! Object-safe (boxed future, no async-trait) so the mock and the real
//! Anthropic client share one interface.
mod mock;
#[cfg(test)]
mod tests;

pub use mock::MockProvider;

use std::future::Future;
use std::pin::Pin;

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

pub trait Provider: Send + Sync {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;
}
```

Create `crates/crew-hive/src/provider/mock.rs`:
```rust
use std::future::Future;
use std::pin::Pin;

use super::{Completion, CompletionRequest, Provider, ProviderError};

/// Deterministic provider for headless tests: returns `reply` and counts tokens
/// by whitespace.
pub struct MockProvider {
    pub reply: String,
}

impl Provider for MockProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let reply = self.reply.clone();
        Box::pin(async move {
            Ok(Completion {
                text: reply.clone(),
                input_tokens: req.prompt.split_whitespace().count() as u32,
                output_tokens: reply.split_whitespace().count() as u32,
            })
        })
    }
}
```

Add `pub mod provider;` to `lib.rs`.

- [ ] **Step 5: Run to verify pass + commit**

Run: `cargo test -p crew-hive provider:: && cargo test -p crew-hive model_id && cargo fmt && cargo clippy -p crew-hive --all-targets` (warning-free).
```bash
git add crates/crew-hive/src/provider crates/crew-hive/src/lib.rs crates/crew-hive/Cargo.toml crates/crew-hive/Cargo.lock crates/crew-hive/src/graph/spec.rs
git commit -m "feat(hive): LLM Provider trait + mock + ModelTier model_id mapping"
```

---

### Task 2: Anthropic provider (reqwest)

**Files:**
- Create: `crates/crew-hive/src/provider/anthropic.rs`
- Modify: `crates/crew-hive/src/provider/mod.rs` (add `mod anthropic;` + re-export `AnthropicProvider`)
- Modify: `crates/crew-hive/src/provider/tests.rs` (add response-parse unit test + an env-gated `#[ignore]` live test)

**Interfaces:**
- Produces: `pub struct AnthropicProvider { client: reqwest::Client, api_key: String }` with:
  - `pub fn from_env() -> Result<Self, ProviderError>` — reads `ANTHROPIC_API_KEY`, errors `MissingKey` if absent/empty.
  - `pub fn new(api_key: String) -> Self`.
  - `Provider` impl: POSTs to `https://api.anthropic.com/v1/messages`, parses the response into `Completion`.
  - `pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError>` — pure parse helper (so it's unit-testable without a network call): extracts the first `content` block of type `text` and `usage.{input,output}_tokens`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/crew-hive/src/provider/tests.rs`:
```rust
#[test]
fn parse_response_extracts_text_and_usage() {
    let body = r#"{
        "content": [{"type": "text", "text": "Hello world"}],
        "usage": {"input_tokens": 12, "output_tokens": 5},
        "stop_reason": "end_turn"
    }"#;
    let c = AnthropicProvider::parse_response(body).unwrap();
    assert_eq!(c.text, "Hello world");
    assert_eq!(c.input_tokens, 12);
    assert_eq!(c.output_tokens, 5);
}

#[test]
fn parse_response_errors_on_api_error_payload() {
    let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#;
    assert!(matches!(
        AnthropicProvider::parse_response(body),
        Err(ProviderError::Api(_))
    ));
}

#[test]
fn from_env_missing_key_errors() {
    // Only assert the error shape when the key is absent; skip otherwise.
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        assert!(matches!(AnthropicProvider::from_env(), Err(ProviderError::MissingKey)));
    }
}

#[tokio::test]
#[ignore = "requires ANTHROPIC_API_KEY; run with --ignored"]
async fn live_anthropic_completion() {
    let p = AnthropicProvider::from_env().expect("key");
    let c = p
        .complete(CompletionRequest {
            model: "claude-haiku-4-5".into(),
            system: Some("Reply with exactly the word: pong".into()),
            prompt: "ping".into(),
            max_tokens: 16,
        })
        .await
        .unwrap();
    assert!(!c.text.is_empty());
    assert!(c.output_tokens > 0);
}
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p crew-hive provider::`
Expected: FAIL — `AnthropicProvider` not defined.

- [ ] **Step 3: Implement the Anthropic provider**

Create `crates/crew-hive/src/provider/anthropic.rs`. Use typed serde structs for the response (a `content` Vec of blocks with `type`/`text`, a `usage` with token counts), plus an error-shape check. Keep ≤ 200 lines.

```rust
use std::future::Future;
use std::pin::Pin;

use serde::Deserialize;

use super::{Completion, CompletionRequest, Provider, ProviderError};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct ApiResp {
    #[serde(default)]
    content: Vec<Block>,
    usage: Option<Usage>,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self { client: reqwest::Client::new(), api_key }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        match std::env::var("ANTHROPIC_API_KEY") {
            Ok(k) if !k.is_empty() => Ok(Self::new(k)),
            _ => Err(ProviderError::MissingKey),
        }
    }

    pub(crate) fn parse_response(body: &str) -> Result<Completion, ProviderError> {
        let r: ApiResp = serde_json::from_str(body).map_err(|e| ProviderError::Decode(e.to_string()))?;
        if r.kind == "error" || r.error.is_some() {
            return Err(ProviderError::Api(body.to_string()));
        }
        let text = r
            .content
            .iter()
            .find(|b| b.kind == "text")
            .map(|b| b.text.clone())
            .unwrap_or_default();
        let usage = r.usage.ok_or_else(|| ProviderError::Decode("missing usage".into()))?;
        Ok(Completion { text, input_tokens: usage.input_tokens, output_tokens: usage.output_tokens })
    }
}

impl Provider for AnthropicProvider {
    fn complete(
        &self,
        req: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>> {
        let client = self.client.clone();
        let key = self.api_key.clone();
        Box::pin(async move {
            let mut body = serde_json::json!({
                "model": req.model,
                "max_tokens": req.max_tokens,
                "messages": [{"role": "user", "content": req.prompt}],
            });
            if let Some(sys) = &req.system {
                body["system"] = serde_json::json!(sys);
            }
            let resp = client
                .post(ENDPOINT)
                .header("x-api-key", key)
                .header("anthropic-version", VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Http(e.to_string()))?;
            let text = resp.text().await.map_err(|e| ProviderError::Http(e.to_string()))?;
            AnthropicProvider::parse_response(&text)
        })
    }
}
```

Add `mod anthropic;` + `pub use anthropic::AnthropicProvider;` to `provider/mod.rs`.

- [ ] **Step 4: Run to verify pass (non-ignored) + commit**

Run: `cargo test -p crew-hive provider:: && cargo fmt && cargo clippy -p crew-hive --all-targets`. The live test stays `#[ignore]`d.
```bash
git add crates/crew-hive/src/provider
git commit -m "feat(hive): Anthropic provider over reqwest (env-gated live test)"
```

---

### Task 3: Planner (stub + LLM-backed)

**Files:**
- Create: `crates/crew-hive/src/planner/mod.rs`
- Create: `crates/crew-hive/src/planner/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod planner;`)

**Interfaces:**
- Consumes: `crate::graph::{TaskGraph, TaskSpec, TaskId, AgentKind, ModelTier, GraphError}`, `crate::provider::{Provider, CompletionRequest, ProviderError}`.
- Produces:
  - `pub enum PlanError { Provider(ProviderError), Parse(String), Graph(GraphError) }` — `Debug`, `Display`, `Error`.
  - `pub trait Planner: Send + Sync { fn plan(&self, goal: &str) -> Pin<Box<dyn Future<Output = Result<TaskGraph, PlanError>> + Send>>; }`
  - `pub struct StubPlanner { pub fanout: usize }` — deterministic: builds `fanout` leaf tasks (deps empty) plus one merge task depending on all leaves; all `AgentKind::Api{system:None}`, `ModelTier::Standard`, prompt = goal. Used by scheduler tests without an LLM.
  - `pub struct LlmPlanner<P: Provider> { pub provider: P, pub tier: ModelTier }` — prompts the provider for a JSON array of tasks and parses it.
  - `pub(crate) fn parse_plan(json: &str) -> Result<TaskGraph, PlanError>` — parses a JSON array `[{"id":N,"title":"...","prompt":"...","deps":[...]}]` into `TaskSpec`s (all `AgentKind::Api{system:None}`, `ModelTier::Standard`) and builds the `TaskGraph`. Pure → unit-testable.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/planner/tests.rs`:
```rust
use super::*;
use crate::graph::TaskId;
use crate::provider::MockProvider;

#[tokio::test]
async fn stub_planner_builds_fanout_plus_merge() {
    let g = StubPlanner { fanout: 3 }.plan("do the thing").await.unwrap();
    assert_eq!(g.len(), 4); // 3 leaves + 1 merge
    // the merge task (highest id) depends on all leaves
    let merge = g.tasks().iter().max_by_key(|t| t.id.0).unwrap();
    assert_eq!(merge.deps.len(), 3);
}

#[test]
fn parse_plan_builds_graph_from_json() {
    let json = r#"[
        {"id": 0, "title": "research", "prompt": "research X", "deps": []},
        {"id": 1, "title": "write", "prompt": "write up X", "deps": [0]}
    ]"#;
    let g = parse_plan(json).unwrap();
    assert_eq!(g.len(), 2);
    assert_eq!(g.get(TaskId(1)).unwrap().deps, vec![TaskId(0)]);
}

#[test]
fn parse_plan_rejects_garbage() {
    assert!(matches!(parse_plan("not json"), Err(PlanError::Parse(_))));
}

#[tokio::test]
async fn llm_planner_parses_provider_json() {
    let reply = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;
    let planner = LlmPlanner { provider: MockProvider { reply: reply.into() }, tier: crate::graph::ModelTier::Standard };
    let g = planner.plan("goal").await.unwrap();
    assert_eq!(g.len(), 1);
}
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p crew-hive planner::`
Expected: FAIL — `planner` not defined.

- [ ] **Step 3: Implement the planner**

Create `crates/crew-hive/src/planner/mod.rs`. The `LlmPlanner::plan` builds a `CompletionRequest` whose system prompt instructs the model to return ONLY a JSON array of `{id,title,prompt,deps}` objects (a task DAG), calls the provider, and runs `parse_plan` on the returned text. `parse_plan` deserializes into an intermediate `Vec<PlanNode>` (serde) then maps to `TaskSpec`. Keep the file ≤ 200 lines (split a `parse.rs` submodule if needed). Use a clear, explicit system prompt, e.g.:
> "You are a task planner. Decompose the user's goal into a JSON array of tasks. Each task is an object with integer `id` (0-based), short `title`, a `prompt` describing the work, and `deps` (array of task ids that must finish first). Return ONLY the JSON array, no prose."

Add `pub mod planner;` to `lib.rs`.

- [ ] **Step 4: Run to verify pass + commit**

Run: `cargo test -p crew-hive planner:: && cargo fmt && cargo clippy -p crew-hive --all-targets`.
```bash
git add crates/crew-hive/src/planner crates/crew-hive/src/lib.rs
git commit -m "feat(hive): Planner trait + stub + LLM-backed JSON task-graph planner"
```

---

### Task 4: Native API agent + full integration test

**Files:**
- Create: `crates/crew-hive/src/apiagent/mod.rs`
- Create: `crates/crew-hive/src/apiagent/tests.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod apiagent;` + re-exports)
- Modify: `crates/crew-hive/tests/engine.rs` (add a planner→scheduler→api-agent integration test using mock provider)

**Interfaces:**
- Consumes: `crate::agent::{Agent, AgentContext}`, `crate::provider::Provider`, `crate::board::TaskResult`, `crate::bus::HiveEvent`, `crate::graph::ModelTier`.
- Produces:
  - `pub struct ApiAgent { provider: std::sync::Arc<dyn Provider>, tier: ModelTier, max_tokens: u32 }` with `pub fn new(provider: Arc<dyn Provider>, tier: ModelTier, max_tokens: u32) -> Self`.
  - `Agent` impl: builds a prompt = the task's `prompt` followed by the gathered dep outputs (`ctx.deps`), calls `provider.complete` with `model = tier.model_id()`, emits `HiveEvent::TokenDelta{input,output}` and `HiveEvent::OutputChunk{text}` (and `HiveEvent::CostDelta` computed from a simple per-tier micros-per-token table), and returns `TaskResult{ task, output: completion.text, success: true }`. On provider error, emits `HiveEvent::Failed` and returns `success: false`.
  - `pub(crate) fn build_prompt(task_prompt: &str, deps: &[TaskResult]) -> String` — pure, testable: appends a "Context from dependencies:" section listing each dep's output.

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/apiagent/tests.rs`:
```rust
use super::*;
use crate::agent::{Agent, AgentContext};
use crate::board::TaskResult;
use crate::bus::{AgentId, EventBus, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskId, TaskSpec};
use crate::provider::MockProvider;
use std::sync::Arc;

fn spec(id: u64) -> TaskSpec {
    TaskSpec { id: TaskId(id), title: "t".into(), agent: AgentKind::Api { system: None }, model: ModelTier::Standard, deps: vec![], prompt: "summarize".into() }
}

#[test]
fn build_prompt_includes_dep_outputs() {
    let deps = vec![TaskResult { task: TaskId(0), output: "alpha".into(), success: true }];
    let p = build_prompt("do it", &deps);
    assert!(p.contains("do it"));
    assert!(p.contains("alpha"));
}

#[tokio::test]
async fn api_agent_completes_and_emits() {
    let bus = EventBus::new(32);
    let mut rx = bus.subscribe();
    let agent = ApiAgent::new(Arc::new(MockProvider { reply: "done".into() }), ModelTier::Standard, 256);
    let ctx = AgentContext { agent: AgentId(0), task: spec(1), deps: vec![], bus: bus.clone() };
    let result = agent.run(ctx).await;
    assert!(result.success);
    assert_eq!(result.output, "done");
    // a token-delta event was emitted
    let mut saw_tokens = false;
    while let Ok(ev) = rx.try_recv() {
        if matches!(ev, HiveEvent::TokenDelta { .. }) { saw_tokens = true; }
    }
    assert!(saw_tokens);
}
```

- [ ] **Step 2: Run to verify fail**

Run: `cargo test -p crew-hive apiagent::`
Expected: FAIL — `apiagent` not defined.

- [ ] **Step 3: Implement the API agent**

Create `crates/crew-hive/src/apiagent/mod.rs` per the interfaces. The per-tier cost table can be a small `match tier` returning micros-USD per 1K tokens (use the skill's pricing: Cheap≈1000/5000 in·out per 1M → integer micros; keep it simple — `micros = input*in_rate + output*out_rate` where rates are per-token micros). Emit `CostDelta { micros_usd }`. Keep ≤ 200 lines (split a `cost.rs` if needed).

Add `pub mod apiagent;` + `pub use apiagent::ApiAgent;` to `lib.rs`. Also re-export the new public types: `pub use provider::{Provider, MockProvider, AnthropicProvider, CompletionRequest, Completion, ProviderError};` and `pub use planner::{Planner, StubPlanner, LlmPlanner, PlanError};`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test -p crew-hive apiagent::`
Expected: PASS.

- [ ] **Step 5: Full integration test — plan → schedule → api-agent (mock provider)**

Append to `crates/crew-hive/tests/engine.rs` a test that: builds a `StubPlanner{fanout:2}` graph (or `LlmPlanner` with a mock), then runs it through `Scheduler` with an `AgentFactory` that makes `ApiAgent`s backed by a shared `MockProvider`, and asserts all tasks `done` + results in the blackboard. This proves the whole brain→scheduler→worker path headlessly.

```rust
#[tokio::test]
async fn plan_then_schedule_with_api_agents() {
    use crew_hive::{Agent, AgentContext, AgentFactory, AgentKind, ApiAgent, Blackboard, EventBus,
        ModelTier, MockProvider, Planner, Scheduler, StubPlanner};
    use std::sync::Arc;

    struct ApiFactory { provider: Arc<MockProvider> }
    impl AgentFactory for ApiFactory {
        fn make(&self, _k: &AgentKind) -> Box<dyn Agent> {
            Box::new(ApiAgent::new(self.provider.clone(), ModelTier::Standard, 256))
        }
    }

    let graph = StubPlanner { fanout: 2 }.plan("build a thing").await.unwrap();
    let n = graph.len();
    let board = Blackboard::new();
    let provider = Arc::new(MockProvider { reply: "ok".into() });
    let out = Scheduler::new(graph, board.clone(), EventBus::new(128), Arc::new(ApiFactory { provider }), 8).run().await;
    assert_eq!(out.done.len(), n);
    assert_eq!(board.result_count().await, n);
}
```
(If `MockProvider` needs to be `Arc`-shared into multiple agents, that's why the factory holds `Arc<MockProvider>` and passes a clone — confirm `ApiAgent::new` takes `Arc<dyn Provider>`; `Arc<MockProvider>` coerces to `Arc<dyn Provider>`.)

- [ ] **Step 6: Full gate + commit**

Run: `cargo fmt && cargo test -p crew-hive && cargo clippy --workspace --all-targets` (workspace warning-free; the live Anthropic test stays `#[ignore]`d).
```bash
git add crates/crew-hive/src/apiagent crates/crew-hive/src/lib.rs crates/crew-hive/tests/engine.rs
git commit -m "feat(hive): native API agent + plan->schedule->agent integration test"
```

---

## Self-Review

- **Spec coverage:** "Planner — LLM-driven decomposer → task graph" → `LlmPlanner` + `parse_plan`. "Native API agents — headless transcript workers, the default scale worker" → `ApiAgent` (futures, no PTY). "Bring-your-own-provider, per agent; model tiering" → `Provider` trait + `ModelTier::model_id` + per-tier cost. Live LLM gated on a key; everything else mock-tested. ✅
- **Placeholder scan:** complete code for provider/mock/anthropic/agent; planner gives complete interfaces + tests + the exact system prompt + parse contract. No TODO/TBD. ✅
- **No new deps:** only `reqwest`/`futures` from the existing workspace set; no `async-trait`. ✅
- **Object safety:** `Provider` and the existing `Agent` both return boxed futures → `Box<dyn _>`/`Arc<dyn _>` valid. ✅
- **Key handling:** `from_env` reads `ANTHROPIC_API_KEY`; live test `#[ignore]`d; key never written to disk or committed. ✅

## Where this sits

Plans 4–5 of the engine. After this, crew-hive can take a goal, decompose it with a real LLM, and run a fan-out of native API agents to completion — the functional core of the swarm. Remaining: the **swarm view** (GUI constellation/heatmap rendering the telemetry), **batch mode + cost governance**, and **remote spill + sidecar bridge**.
