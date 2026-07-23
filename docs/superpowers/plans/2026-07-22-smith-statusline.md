# Claude-Style /smith Statusline Footer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the /smith pane's summary footer with a colored 3-line Claude-Code-style statusline (`model | branch | $cost | in/out`, rolling 5h/7d windows + bars, routing-mode line), plus `├`/`└` tree connectors for multi-reply task chains.

**Architecture:** Cost is computed broker-side at the `ApiAdapter` boundary (the only place that knows the model), carried on the existing `Usage` struct so it flows through `Hop`/`RunStats` untouched, and shipped to the GUI via three new serde-defaulted fields on `PluginEvent::Stats`. The GUI accumulates in/out/cost on `ChatPane`, persists turn totals to a `usage.jsonl` ledger (singleton module, injected-clock window math), and renders everything in a rewritten `chatsummary.rs`.

**Tech Stack:** Rust workspace (crates: crew-hive, crew-plugin, crew-app, crew-theme, crew-render), serde/serde_json, toml, no new dependencies.

**Spec:** `docs/superpowers/specs/2026-07-22-smith-statusline-design.md`

## Global Constraints

- Colors are `(u8, u8, u8)` tuples from `crew_theme::theme()`; accents come from `theme().ansi[9..=14]` — never hardcode RGB in crew-app.
- Never run git or blocking I/O in render paths (winit main thread); ledger writes happen where Stats events are applied (poll path, one small append) and are `cfg!(test)`-guarded like `CrewConfig::save`.
- All money is `u64` micro-USD (1 USD = 1_000_000), matching `crew-hive/src/govern` convention.
- Old GUI ↔ new broker and new GUI ↔ old broker must both keep working: every new protocol field is `#[serde(default)]`.
- `cargo fmt` + `cargo check` run in the pre-commit hook; commits fail if formatting is off.
- Commit messages end with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

### Task 1: Pricing table in crew-hive

**Files:**
- Create: `crates/crew-hive/src/pricing.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod pricing;`)

**Interfaces:**
- Produces: `crew_hive::pricing::cost_microusd(model: &str, input_tokens: u32, output_tokens: u32) -> u64` (0 = unknown model). Used by Task 3 (`ApiAdapter`).

- [ ] **Step 1: Write the failing tests**

Create `crates/crew-hive/src/pricing.rs`:

```rust
//! Approximate per-model API pricing, used to attach a dollar cost to token
//! usage when the provider doesn't report one exactly (OpenRouter does; see
//! `provider::openai_http`). Rates are micro-USD per 1M tokens, matched by
//! substring on the model slug (longest pattern first), so provider prefixes
//! (`anthropic/claude-sonnet-5`) and date suffixes both hit. Unknown models
//! cost 0 — the footer hides the `$` segment rather than invent a number.

/// (slug substring, input µ$/Mtok, output µ$/Mtok). Approximate list prices,
/// 2026-07. Order does not matter — the longest matching pattern wins.
const RATES: &[(&str, u64, u64)] = &[
    // Anthropic
    ("claude-opus", 15_000_000, 75_000_000),
    ("claude-sonnet", 3_000_000, 15_000_000),
    ("claude-haiku", 1_000_000, 5_000_000),
    ("claude-fable", 15_000_000, 75_000_000),
    // Qwen / DashScope
    ("qwen3-coder-plus", 1_000_000, 5_000_000),
    ("qwen3-coder-flash", 300_000, 1_500_000),
    ("qwen3-coder", 1_000_000, 5_000_000),
    ("qwen-max", 1_600_000, 6_400_000),
    ("qwen-plus", 400_000, 1_200_000),
    ("qwen-turbo", 50_000, 200_000),
    // OpenAI
    ("gpt-4o-mini", 150_000, 600_000),
    ("gpt-4o", 2_500_000, 10_000_000),
    ("gpt-4.1-mini", 400_000, 1_600_000),
    ("gpt-4.1", 2_000_000, 8_000_000),
    // DeepSeek
    ("deepseek-reasoner", 550_000, 2_190_000),
    ("deepseek", 270_000, 1_100_000),
    // Moonshot / Kimi
    ("kimi-k2", 600_000, 2_500_000),
];

/// Approximate cost of one reply in micro-USD; 0 when the model is unknown.
pub fn cost_microusd(model: &str, input_tokens: u32, output_tokens: u32) -> u64 {
    let m = model.to_ascii_lowercase();
    let Some((_, in_rate, out_rate)) = RATES
        .iter()
        .filter(|(pat, _, _)| m.contains(pat))
        .max_by_key(|(pat, _, _)| pat.len())
    else {
        return 0;
    };
    (in_rate * u64::from(input_tokens) + out_rate * u64::from(output_tokens)) / 1_000_000
}

#[cfg(test)]
mod tests {
    use super::cost_microusd;

    #[test]
    fn longest_pattern_wins() {
        // qwen3-coder-flash must match its own cheaper rate, not qwen3-coder.
        // 1M in at $0.3/Mtok = 300_000 µ$.
        assert_eq!(cost_microusd("qwen3-coder-flash", 1_000_000, 0), 300_000);
        assert_eq!(cost_microusd("qwen3-coder-plus", 1_000_000, 0), 1_000_000);
    }

    #[test]
    fn provider_prefix_and_case_are_ignored() {
        // $3/Mtok in + $15/Mtok out: 10k in + 1k out = 30_000 + 15_000 µ$.
        assert_eq!(
            cost_microusd("anthropic/Claude-Sonnet-5", 10_000, 1_000),
            45_000
        );
    }

    #[test]
    fn unknown_model_costs_zero() {
        assert_eq!(cost_microusd("mock-model", 1_000_000, 1_000_000), 0);
        assert_eq!(cost_microusd("", 5, 5), 0);
    }

    #[test]
    fn zero_tokens_cost_zero() {
        assert_eq!(cost_microusd("claude-opus-4-8", 0, 0), 0);
    }
}
```

- [ ] **Step 2: Wire the module and verify tests fail first, then pass**

Add to `crates/crew-hive/src/lib.rs` next to the other `pub mod` lines:

```rust
pub mod pricing;
```

Run: `cargo test -p crew-hive pricing -- --nocapture`
Expected: 4 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/crew-hive/src/pricing.rs crates/crew-hive/src/lib.rs
git commit -m "feat(hive): per-model pricing table (micro-USD per Mtok)"
```

---

### Task 2: Exact OpenRouter cost through `Completion`

**Files:**
- Modify: `crates/crew-hive/src/provider/mod.rs` (`Completion` struct, ~line 60)
- Modify: `crates/crew-hive/src/provider/openai_http.rs` (`Usage`/`parse_response` ~line 403-440, `parse_sse_line` ~line 366-389, `SseItem`)
- Modify: `crates/crew-hive/src/provider/openrouter.rs` (`build_body` ~line 61, provider struct)
- Modify: `crates/crew-hive/src/provider/anthropic.rs`, `mock.rs`, plus any other `Completion { ... }` construction sites `cargo check` reports
- Test: inline `#[cfg(test)]` in `openai_http.rs` / existing `openrouter_tests.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `Completion { text: String, input_tokens: u32, output_tokens: u32, cost_microusd: u64 }` — `cost_microusd` is the provider-reported exact cost, 0 when the API didn't report one. Task 3 reads it.

- [ ] **Step 1: Add the field**

In `crates/crew-hive/src/provider/mod.rs`:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Completion {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Exact provider-reported cost in micro-USD (OpenRouter `usage.cost`);
    /// 0 when the provider doesn't report cost.
    pub cost_microusd: u64,
}
```

Run: `cargo check -p crew-hive 2>&1 | grep "missing field"` and add `cost_microusd: 0,` at every listed construction site (anthropic.rs, mock.rs, openai_http.rs, remoteagent, tests). Only `parse_response` gets a real value (Step 2).

- [ ] **Step 2: Parse `usage.cost` (dollars, f64) in the non-streaming path**

In `openai_http.rs`, extend the deserialize struct and `parse_response`:

```rust
#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    /// OpenRouter-only: exact request cost in USD when the request asked
    /// for it (`usage: {include: true}`). Absent everywhere else.
    #[serde(default)]
    cost: f64,
}
```

and in `parse_response`'s `Ok(Completion { ... })`:

```rust
    Ok(Completion {
        text,
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
        cost_microusd: (usage.cost * 1_000_000.0) as u64,
    })
```

- [ ] **Step 3: Parse cost in the SSE path**

Change `SseItem::Usage(u64, u64)` to `SseItem::Usage(u64, u64, u64)` (in, out, cost µ$) and in `parse_sse_line`:

```rust
    if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
        let i = u["prompt_tokens"].as_u64().unwrap_or(0);
        let o = u["completion_tokens"].as_u64().unwrap_or(0);
        let cost = (u["cost"].as_f64().unwrap_or(0.0) * 1_000_000.0) as u64;
        if i > 0 || o > 0 {
            return SseItem::Usage(i, o, cost);
        }
    }
```

Fix the match sites `cargo check` reports (the streaming driver that folds `SseItem::Usage` into the final `Completion` — carry the cost through the same way as the token counts).

- [ ] **Step 4: Request cost from OpenRouter only**

In `openrouter.rs`, the provider stores its endpoint (see `with_endpoint`). Add a helper and use it in `build_body`:

```rust
/// OpenRouter reports exact request cost when asked. Other OpenAI-compatible
/// endpoints (DashScope!) may 400 on the unknown `usage` field, so the ask is
/// gated on the real openrouter.ai endpoint.
fn wants_cost(endpoint: &str) -> bool {
    endpoint.contains("openrouter.ai")
}
```

Thread a `report_cost: bool` parameter into `build_body` (callers pass `wants_cost(&self.endpoint)` — match the actual field name found in the struct):

```rust
fn build_body(
    model: &str,
    req: &CompletionRequest,
    messages: &[serde_json::Value],
    report_cost: bool,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": req.max_tokens,
        "messages": messages,
    });
    if report_cost {
        body["usage"] = serde_json::json!({"include": true});
    }
    body
}
```

- [ ] **Step 5: Tests**

Add to the existing test module in `openai_http.rs` (mirror the neighboring parse tests' style):

```rust
#[test]
fn parse_response_reads_openrouter_cost() {
    let body = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5,"cost":0.000129}}"#;
    let c = parse_response(body).unwrap();
    assert_eq!(c.cost_microusd, 129);
}

#[test]
fn parse_response_without_cost_is_zero() {
    let body = r#"{"choices":[{"message":{"content":"hi"}}],
        "usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
    assert_eq!(parse_response(body).unwrap().cost_microusd, 0);
}
```

And in `openrouter_tests.rs` (match existing body-building test style):

```rust
#[test]
fn openrouter_body_asks_for_cost_but_custom_endpoint_does_not() {
    // exact assertion shape per existing build_body tests: the JSON has
    // body["usage"]["include"] == true only when report_cost is true.
}
```

(Write the real assertions against `build_body(..., true)` / `build_body(..., false)` outputs.)

Run: `cargo test -p crew-hive provider`
Expected: all pass, including new ones.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-hive/src
git commit -m "feat(hive): carry exact OpenRouter cost on Completion"
```

---

### Task 3: Cost on broker `Usage` + protocol `Stats` fields

**Files:**
- Modify: `crates/crew-plugin/src/broker/adapter.rs` (`Usage`, ~line 28)
- Modify: `crates/crew-plugin/src/broker/apiadapter.rs` (both `Usage { ... }` sites, ~lines 130-135 and 173-178)
- Modify: `crates/crew-plugin/src/protocol.rs` (`Stats` variant)
- Modify: `crates/crew-plugin/src/broker/hop.rs` (`RunStats`, ~line 35)
- Modify: `crates/crew-plugin/src/broker/engine.rs` (~lines 143-145, 161-164), `toolcall.rs` (~193-196), `relay.rs` (3 emit sites), `fan.rs` (3 emit sites)
- Test: protocol tests in `protocol.rs`, existing broker tests

**Interfaces:**
- Consumes: `crew_hive::pricing::cost_microusd` (Task 1), `Completion.cost_microusd` (Task 2).
- Produces: `Usage { input_tokens: u32, output_tokens: u32, cost_microusd: u64 }`; `RunStats { exchanges, approx_tokens, real_tokens, tok_in: u64, tok_out: u64, cost_microusd: u64 }`; `PluginEvent::Stats { exchanges, tokens, agent, ms, ctx, tok_in: u64, tok_out: u64, cost_microusd: u64 }`. Task 4 (GUI) consumes the Stats fields.

- [ ] **Step 1: Protocol round-trip test first**

Add to the test module in `protocol.rs`:

```rust
#[test]
fn stats_roundtrips_cost_fields_and_defaults_when_missing() {
    // Old-broker payload (no new fields) must still decode.
    let old = r#"{"type":"stats","exchanges":3,"tokens":950}"#;
    match serde_json::from_str::<PluginEvent>(old).unwrap() {
        PluginEvent::Stats { tok_in, tok_out, cost_microusd, .. } => {
            assert_eq!((tok_in, tok_out, cost_microusd), (0, 0, 0));
        }
        other => panic!("wrong variant: {other:?}"),
    }
    // New payload round-trips.
    let ev = PluginEvent::Stats {
        exchanges: 1,
        tokens: 950,
        agent: String::new(),
        ms: 0,
        ctx: 0,
        tok_in: 900,
        tok_out: 50,
        cost_microusd: 12_345,
    };
    let s = serde_json::to_string(&ev).unwrap();
    match serde_json::from_str::<PluginEvent>(&s).unwrap() {
        PluginEvent::Stats { tok_in, tok_out, cost_microusd, .. } => {
            assert_eq!((tok_in, tok_out, cost_microusd), (900, 50, 12_345));
        }
        other => panic!("wrong variant: {other:?}"),
    }
}
```

Run: `cargo test -p crew-plugin protocol` — expected: FAIL (missing fields).

- [ ] **Step 2: Add the protocol fields**

In `protocol.rs`, extend `Stats`:

```rust
    Stats {
        exchanges: u32,
        tokens: u64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        agent: String,
        #[serde(default)]
        ms: u64,
        /// The reply's real prompt size in tokens — the agent's live context
        /// fill — when the backend reports usage; 0 = unknown.
        #[serde(default)]
        ctx: u64,
        /// Prompt/completion token split for the same usage `tokens` reports,
        /// and the broker-computed cost in micro-USD (0 = unpriced model).
        /// All serde-defaulted so old payloads still decode.
        #[serde(default)]
        tok_in: u64,
        #[serde(default)]
        tok_out: u64,
        #[serde(default)]
        cost_microusd: u64,
    },
```

- [ ] **Step 3: Cost onto `Usage` at the adapter boundary**

`adapter.rs`:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Micro-USD for this reply: the provider's exact figure when reported,
    /// else the pricing-table estimate for the adapter's model; 0 = unknown.
    pub cost_microusd: u64,
}
```

`apiadapter.rs`, both construction sites (the `Completion c` is in scope at each):

```rust
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
```

Fix remaining `Usage { ... }` construction sites `cargo check -p crew-plugin` reports with `cost_microusd: 0` (CLI adapters / tests / defaults are covered by `Default`).

- [ ] **Step 4: Accumulate through `RunStats` and fill every emit site**

`hop.rs`:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    pub exchanges: u32,
    pub approx_tokens: usize,
    pub real_tokens: usize,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
}
```

`engine.rs` (both accumulation sites, next to the existing `real_tokens` line):

```rust
            stats.real_tokens += (usage.input_tokens + usage.output_tokens) as usize;
            stats.tok_in += u64::from(usage.input_tokens);
            stats.tok_out += u64::from(usage.output_tokens);
            stats.cost_microusd += usage.cost_microusd;
```

`toolcall.rs` (same three lines after its `real_tokens` accumulation, using `u`).

`relay.rs::reply_stat` (per-reply stat — split + cost ride along):

```rust
fn reply_stat(agent: &str, d: Duration, hop: &Hop) -> PluginEvent {
    PluginEvent::Stats {
        exchanges: 0,
        tokens: (hop.usage.input_tokens + hop.usage.output_tokens) as u64,
        agent: agent.to_string(),
        ms: d.as_millis() as u64,
        ctx: hop.usage.input_tokens as u64,
        tok_in: u64::from(hop.usage.input_tokens),
        tok_out: u64::from(hop.usage.output_tokens),
        cost_microusd: hop.usage.cost_microusd,
    }
}
```

`relay.rs` turn total (~line 83): `tok_in: stats.tok_in, tok_out: stats.tok_out, cost_microusd: stats.cost_microusd`. The dangling-timing close-out (~line 68) and `fan.rs`'s error arm get zeros.

`fan.rs` success arm (has `u` in scope): `tok_in: u64::from(u.input_tokens), tok_out: u64::from(u.output_tokens), cost_microusd: u.cost_microusd`; accumulate the same three into locals next to `real_tokens` and fill the combined total emit (~line 130) from them.

- [ ] **Step 5: Test and commit**

Run: `cargo test -p crew-plugin`
Expected: all pass, including Step 1's test.

```bash
git add crates/crew-plugin/src
git commit -m "feat(broker): in/out token split + micro-USD cost on Stats events"
```

---

### Task 4: GUI pane accumulates in/out/cost

**Files:**
- Modify: `crates/crew-app/src/chat.rs` (struct ~line 30, constructor ~line 86, `poll` match arm ~line 148)
- Modify: `crates/crew-app/src/chatflow.rs` (`absorb_stats`, line 25)
- Test: `crates/crew-app/src/chatflow_tests.rs` (or wherever `absorb_stats` tests live — check for an existing `#[path]` test module and extend it)

**Interfaces:**
- Consumes: `PluginEvent::Stats` new fields (Task 3).
- Produces: `ChatPane { pub(crate) tok_in: u64, pub(crate) tok_out: u64, pub(crate) cost_microusd: u64, ... }` and `absorb_stats(&mut self, tokens: u64, agent: String, ms: u64, ctx: u64, tok_in: u64, tok_out: u64, cost_microusd: u64)`. Also calls `crate::usageledger::record(tok_in, tok_out, cost_microusd)` on turn totals — Task 5 provides it; UNTIL Task 5 lands, leave the call OUT (this task must compile alone; Task 5 adds the one-line call).

- [ ] **Step 1: Failing test**

Next to the existing absorb/stats tests (find them: `grep -rn "absorb_stats" crates/crew-app/src --include="*_tests.rs"`; if none exist, add a test module `chatflow_tests.rs` wired with `#[cfg(test)] #[path = "chatflow_tests.rs"] mod tests;` at the bottom of `chatflow.rs`, matching the `chatsummary.rs` pattern):

```rust
#[test]
fn turn_total_accumulates_split_and_cost() {
    let mut pane = test_pane(); // mirror however neighboring tests build one
    pane.absorb_stats(950, String::new(), 0, 0, 900, 50, 129);
    pane.absorb_stats(100, String::new(), 0, 0, 90, 10, 21);
    assert_eq!(pane.tok_in, 990);
    assert_eq!(pane.tok_out, 60);
    assert_eq!(pane.cost_microusd, 150);
    // Per-agent reply stats must NOT double-count into session totals.
    pane.absorb_stats(500, "coder".into(), 800, 400, 450, 50, 60);
    assert_eq!(pane.tok_in, 990);
}
```

Run: `cargo test -p crew-app chatflow` — expected: FAIL (no such fields/params).

- [ ] **Step 2: Implement**

`chat.rs` struct (after `tokens`):

```rust
    /// Session prompt/completion token split and micro-USD cost, from
    /// turn-level `Stats` events (same cadence as `tokens`).
    pub(crate) tok_in: u64,
    pub(crate) tok_out: u64,
    pub(crate) cost_microusd: u64,
```

Constructor: `tok_in: 0, tok_out: 0, cost_microusd: 0,`.

`poll` match arm:

```rust
                PluginEvent::Stats {
                    tokens,
                    agent,
                    ms,
                    ctx,
                    tok_in,
                    tok_out,
                    cost_microusd,
                    ..
                } => self.absorb_stats(tokens, agent, ms, ctx, tok_in, tok_out, cost_microusd),
```

`chatflow.rs`:

```rust
pub(crate) fn absorb_stats(
    &mut self,
    tokens: u64,
    agent: String,
    ms: u64,
    ctx: u64,
    tok_in: u64,
    tok_out: u64,
    cost_microusd: u64,
) {
    if agent.is_empty() {
        self.tokens = self.tokens.saturating_add(tokens);
        self.turns = self.turns.saturating_add(1);
        self.tok_in = self.tok_in.saturating_add(tok_in);
        self.tok_out = self.tok_out.saturating_add(tok_out);
        self.cost_microusd = self.cost_microusd.saturating_add(cost_microusd);
    } else {
        // A follow-up event reporting no usage must not clear a known fill,
        // so only a nonzero `ctx` overwrites.
        if ctx > 0 {
            self.ctx.insert(agent.clone(), ctx);
        }
        let e = self.agent_stats.entry(agent).or_default();
        e.0 = e.0.saturating_add(1);
        e.1 = e.1.saturating_add(ms);
    }
}
```

Fix any other `absorb_stats` callers `cargo check -p crew-app` reports (pass zeros in tests that don't care).

- [ ] **Step 3: Test and commit**

Run: `cargo test -p crew-app`
Expected: all pass.

```bash
git add crates/crew-app/src
git commit -m "feat(app): pane accumulates in/out token split and cost"
```

---

### Task 5: Usage ledger with 5h/7d rolling windows

**Files:**
- Create: `crates/crew-app/src/usageledger.rs`
- Create: `crates/crew-app/src/usageledger_tests.rs`
- Modify: `crates/crew-app/src/main.rs` or wherever `mod` declarations live (add `mod usageledger;` — find the module list: `grep -n "mod chatsummary" crates/crew-app/src/*.rs`)
- Modify: `crates/crew-app/src/config.rs` (budget options)
- Modify: `crates/crew-app/src/handler.rs:104` area (init after `CrewConfig::load()`)
- Modify: `crates/crew-app/src/chatflow.rs` (one `record` call)

**Interfaces:**
- Consumes: `ChatPane` turn totals (Task 4).
- Produces:
  - `pub(crate) struct WindowStat { pub left_ms: u64, pub spent: u64, pub budget: u64 }`
  - `pub(crate) struct Windows { pub five_h: Option<WindowStat>, pub seven_d: Option<WindowStat> }`
  - `pub(crate) fn init(budget_5h: u64, budget_7d: u64)` — load file + set budgets (call once at startup)
  - `pub(crate) fn record(tok_in: u64, tok_out: u64, cost_microusd: u64)` — append to memory + file (file write skipped under `cfg!(test)`)
  - `pub(crate) fn windows(now_ms: u64) -> Windows` — Task 6's footer reads this
  - Pure core (directly unit-tested): `pub(crate) struct Ledger { entries: Vec<Entry>, budget_5h: u64, budget_7d: u64 }` with `fn window(&self, now_ms: u64, span_ms: u64, budget: u64) -> Option<WindowStat>`
- Config: `usage_budget_5h: u64` (default 5_000_000), `usage_budget_7d: u64` (default 25_000_000), clamped to `10_000..=u64::MAX`.

- [ ] **Step 1: Failing window-math tests**

Create `crates/crew-app/src/usageledger_tests.rs`:

```rust
use super::*;

const H: u64 = 3_600_000;

fn ledger(entries: &[(u64, u64)]) -> Ledger {
    // (ts_ms, tokens) — split evenly across in/out; cost 0.
    let entries = entries
        .iter()
        .map(|&(ts_ms, tok)| Entry {
            ts_ms,
            tok_in: tok / 2,
            tok_out: tok - tok / 2,
            cost_microusd: 0,
        })
        .collect();
    Ledger {
        entries,
        budget_5h: 1_000,
        budget_7d: 10_000,
    }
}

#[test]
fn no_usage_means_no_open_window() {
    assert!(ledger(&[]).window(100 * H, 5 * H, 1_000).is_none());
}

#[test]
fn block_opens_at_first_use_and_counts_spend_and_time_left() {
    // First use at t=10h opens a 5h block [10h, 15h).
    let l = ledger(&[(10 * H, 300), (12 * H, 200)]);
    let w = l.window(13 * H, 5 * H, 1_000).unwrap();
    assert_eq!(w.left_ms, 2 * H);
    assert_eq!(w.spent, 500);
    assert_eq!(w.budget, 1_000);
}

#[test]
fn expired_block_closes_and_next_use_opens_a_new_one() {
    // Block [10h,15h) expired; nothing since → no open window at t=16h.
    let l = ledger(&[(10 * H, 300)]);
    assert!(l.window(16 * H, 5 * H, 1_000).is_none());
    // A later entry at t=17h opens [17h,22h) containing only its own spend.
    let l = ledger(&[(10 * H, 300), (17 * H, 50)]);
    let w = l.window(18 * H, 5 * H, 1_000).unwrap();
    assert_eq!(w.left_ms, 4 * H);
    assert_eq!(w.spent, 50);
}

#[test]
fn chained_blocks_reset_on_boundaries_not_on_gaps() {
    // Use at 10h opens [10h,15h); use at 16h (after expiry) opens [16h,21h).
    let l = ledger(&[(10 * H, 300), (16 * H, 100), (20 * H, 100)]);
    let w = l.window(20 * H, 5 * H, 1_000).unwrap();
    assert_eq!(w.spent, 200);
    assert_eq!(w.left_ms, H);
}

#[test]
fn prune_drops_entries_older_than_seven_days() {
    let mut l = ledger(&[(0, 100), (8 * 24 * H, 100)]);
    l.prune(8 * 24 * H + H);
    assert_eq!(l.entries.len(), 1);
}
```

Run: `cargo test -p crew-app usageledger` — expected: FAIL (module missing).

- [ ] **Step 2: Implement the module**

Create `crates/crew-app/src/usageledger.rs`:

```rust
//! Crew's own usage history, for the footer's Claude-style rolling windows.
//! A window opens at the first request after the previous window expired
//! (Claude session semantics) — a 5-hour and a 7-day block — and the footer
//! shows time left + budget spent for each. Entries persist as JSON lines in
//! `usage.jsonl` beside `config.toml`, pruned past 7 days on load. The
//! process-wide singleton aggregates across panes (the GUI is one process;
//! brokers are per-pane children and would race on the file).
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub(crate) const FIVE_H_MS: u64 = 5 * 3_600_000;
pub(crate) const SEVEN_D_MS: u64 = 7 * 24 * 3_600_000;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Entry {
    pub ts_ms: u64,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowStat {
    /// Milliseconds until this block rolls over.
    pub left_ms: u64,
    /// Tokens (in+out) spent inside the block so far.
    pub spent: u64,
    pub budget: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Windows {
    pub five_h: Option<WindowStat>,
    pub seven_d: Option<WindowStat>,
}

#[derive(Default)]
pub(crate) struct Ledger {
    pub(crate) entries: Vec<Entry>,
    pub(crate) budget_5h: u64,
    pub(crate) budget_7d: u64,
}

impl Ledger {
    /// The open `span_ms` block at `now_ms`, if any. Blocks chain: the first
    /// entry opens one; an entry past a block's end opens the next at its own
    /// timestamp (not at the boundary), so idle gaps don't tick windows over.
    pub(crate) fn window(&self, now_ms: u64, span_ms: u64, budget: u64) -> Option<WindowStat> {
        let mut start: Option<u64> = None;
        for e in &self.entries {
            match start {
                None => start = Some(e.ts_ms),
                Some(s) if e.ts_ms >= s + span_ms => start = Some(e.ts_ms),
                Some(_) => {}
            }
        }
        let s = start?;
        if now_ms >= s + span_ms {
            return None; // last block expired with no use since
        }
        let spent = self
            .entries
            .iter()
            .filter(|e| e.ts_ms >= s)
            .map(|e| e.tok_in + e.tok_out)
            .sum();
        Some(WindowStat {
            left_ms: s + span_ms - now_ms,
            spent,
            budget,
        })
    }

    pub(crate) fn windows(&self, now_ms: u64) -> Windows {
        Windows {
            five_h: self.window(now_ms, FIVE_H_MS, self.budget_5h),
            seven_d: self.window(now_ms, SEVEN_D_MS, self.budget_7d),
        }
    }

    /// Drop entries older than the 7d horizon — nothing renders them again.
    pub(crate) fn prune(&mut self, now_ms: u64) {
        let floor = now_ms.saturating_sub(SEVEN_D_MS);
        self.entries.retain(|e| e.ts_ms >= floor);
    }
}

/// `usage.jsonl` beside the config file.
fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("usage.jsonl"))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static LEDGER: Mutex<Option<Ledger>> = Mutex::new(None);

/// Load the ledger and set budgets. Call once at startup, after config load.
pub(crate) fn init(budget_5h: u64, budget_7d: u64) {
    let mut l = Ledger {
        entries: Vec::new(),
        budget_5h,
        budget_7d,
    };
    if !cfg!(test) {
        if let Some(p) = path() {
            if let Ok(text) = std::fs::read_to_string(&p) {
                l.entries = text
                    .lines()
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect();
            }
            l.prune(now_ms());
            // Rewrite pruned so the file doesn't grow without bound.
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let body: String = l
                .entries
                .iter()
                .filter_map(|e| serde_json::to_string(e).ok())
                .map(|s| s + "\n")
                .collect();
            let _ = std::fs::write(&p, body);
        }
    }
    *LEDGER.lock().unwrap_or_else(|e| e.into_inner()) = Some(l);
}

/// Record one turn's usage: in-memory always, appended to disk outside tests.
pub(crate) fn record(tok_in: u64, tok_out: u64, cost_microusd: u64) {
    if tok_in == 0 && tok_out == 0 {
        return; // mock/CLI backends report no usage — nothing to window
    }
    let e = Entry {
        ts_ms: now_ms(),
        tok_in,
        tok_out,
        cost_microusd,
    };
    let mut guard = LEDGER.lock().unwrap_or_else(|e| e.into_inner());
    guard.get_or_insert_with(Ledger::default).entries.push(e);
    if cfg!(test) {
        return;
    }
    if let Some(p) = path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let (Ok(line), Ok(mut f)) = (
            serde_json::to_string(&e),
            std::fs::OpenOptions::new().create(true).append(true).open(&p),
        ) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// The current rolling windows, for the footer. Cheap: a scan over ≤7d of
/// per-turn entries under a mutex — fine on the render path.
pub(crate) fn windows(now_ms: u64) -> Windows {
    let guard = LEDGER.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map(|l| l.windows(now_ms)).unwrap_or_default()
}

#[cfg(test)]
#[path = "usageledger_tests.rs"]
mod tests;
```

Register the module next to `mod chatsummary;` (same file that declares it).

- [ ] **Step 3: Config budgets**

In `config.rs`, following the `notify_min_secs` pattern exactly:

```rust
fn default_usage_budget_5h() -> u64 {
    5_000_000
}
fn default_usage_budget_7d() -> u64 {
    25_000_000
}
```

Fields on `CrewConfig`:

```rust
    /// Token budgets for the footer's rolling usage windows (the `%` the
    /// bars are drawn against). Approximate by nature — tune to taste.
    #[serde(default = "default_usage_budget_5h")]
    pub usage_budget_5h: u64,
    #[serde(default = "default_usage_budget_7d")]
    pub usage_budget_7d: u64,
```

`impl Default`: `usage_budget_5h: default_usage_budget_5h(), usage_budget_7d: default_usage_budget_7d(),`. In `clamped()`: `usage_budget_5h: self.usage_budget_5h.max(10_000), usage_budget_7d: self.usage_budget_7d.max(10_000),`.

Config test (next to the `notify_min_secs` clamp test):

```rust
#[test]
fn usage_budgets_default_and_clamp() {
    let cfg = CrewConfig::from_toml_str("");
    assert_eq!(cfg.usage_budget_5h, 5_000_000);
    assert_eq!(cfg.usage_budget_7d, 25_000_000);
    let cfg = CrewConfig::from_toml_str("usage_budget_5h = 1\n");
    assert_eq!(cfg.usage_budget_5h, 10_000);
}
```

- [ ] **Step 4: Wire startup + record**

`handler.rs`, right after `let config = CrewConfig::load();` (line 104):

```rust
    crate::usageledger::init(config.usage_budget_5h, config.usage_budget_7d);
```

`chatflow.rs::absorb_stats`, in the `agent.is_empty()` branch, after the cost accumulation line:

```rust
        crate::usageledger::record(tok_in, tok_out, cost_microusd);
```

- [ ] **Step 5: Test and commit**

Run: `cargo test -p crew-app`
Expected: all pass (usageledger 5 new + config 1 new).

```bash
git add crates/crew-app/src
git commit -m "feat(app): usage ledger with 5h/7d rolling windows and config budgets"
```

---

### Task 6: The 3-line colored footer

**Files:**
- Modify: `crates/crew-app/src/chatsummary.rs` (rewrite the builders; keep `summary_rows`/`summary_cells` names and the `place_row` render loop)
- Modify: `crates/crew-app/src/chatsummary_tests.rs` (rewrite)
- Modify: `crates/crew-app/src/chatinput.rs` (`PLACEHOLDER_HINT`, `mention_len` visibility)

**Interfaces:**
- Consumes: `ChatPane.{tok_in,tok_out,cost_microusd,tokens,agents,ctx,git_branch,input}` (Task 4), `usageledger::windows` (Task 5), `chatinput::relay_target` (added here).
- Produces:
  - `pub(crate) fn footer_lines(fc: &FooterCtx, cols: usize) -> Vec<Vec<(char, (u8, u8, u8))>>` — pure, fully testable
  - `pub(crate) struct FooterCtx<'a>` (fields below)
  - `pub(crate) fn relay_target<'a>(input: &'a str, agents: &[AgentInfo]) -> Option<&'a str>` in `chatinput.rs`
  - `summary_rows`/`summary_cells` keep their existing signatures (callers in `chatplace.rs:106` and `chatview.rs:185` untouched).

- [ ] **Step 1: Failing tests (rewrite `chatsummary_tests.rs`)**

Keep the `agent`/`ctx` helpers; replace block tests with footer tests. Text assertions flatten chars; color assertions probe segments:

```rust
use super::*;
use crew_plugin::AgentInfo;
use std::collections::HashMap;

fn agent(name: &str, model: &str) -> AgentInfo {
    AgentInfo {
        name: name.into(),
        role: String::new(),
        model: model.into(),
    }
}

fn ctx(pairs: &[(&str, u64)]) -> HashMap<String, u64> {
    pairs.iter().map(|(n, v)| (n.to_string(), *v)).collect()
}

fn text(line: &[(char, (u8, u8, u8))]) -> String {
    line.iter().map(|(c, _)| *c).collect()
}

fn fc<'a>(
    agents: &'a [AgentInfo],
    ctxm: &'a HashMap<String, u64>,
) -> FooterCtx<'a> {
    FooterCtx {
        agents,
        ctx: ctxm,
        tok_in: 41_600,
        tok_out: 314,
        cost_microusd: 129_000, // $0.129
        branch: Some("main"),
        input: "",
        windows: crate::usageledger::Windows {
            five_h: Some(crate::usageledger::WindowStat {
                left_ms: (3 * 60 + 52) * 60_000, // 3h52m
                spent: 150_000,
                budget: 5_000_000, // 3%
            }),
            seven_d: Some(crate::usageledger::WindowStat {
                left_ms: (3 * 24 + 23) * 3_600_000, // 3d23h
                spent: 0,
                budget: 25_000_000,
            }),
        },
    }
}

#[test]
fn line1_shows_model_branch_cost_and_split() {
    let agents = [agent("smith", "qwen/qwen3-coder-plus")];
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 120);
    assert_eq!(
        text(&lines[0]),
        "qwen3-coder-plus | main | $0.129 | 41.6k in / 314 out"
    );
}

#[test]
fn line1_hides_cost_when_unpriced_but_always_shows_tokens() {
    let mut f = fc(&[], &HashMap::new());
    f.cost_microusd = 0;
    f.tok_in = 0;
    f.tok_out = 0;
    f.branch = None;
    let lines = footer_lines(&f, 120);
    assert_eq!(text(&lines[0]), "0 in / 0 out");
}

#[test]
fn line2_shows_countdowns_and_bars() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    // opus limit 200k, 100k used → ctx bar 50%.
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 120);
    let l2 = text(&lines[1]);
    assert!(l2.starts_with("5h:3h52m | 7d:3d23h | "), "{l2}");
    assert!(l2.contains("3% (5h)"), "{l2}");
    assert!(l2.ends_with("50% (ctx)"), "{l2}");
}

#[test]
fn line2_dashes_when_no_window_and_drops_ctx_without_agents() {
    let mut f = fc(&[], &HashMap::new());
    f.windows = crate::usageledger::Windows::default();
    let l2 = text(&footer_lines(&f, 120)[1]);
    assert_eq!(l2, "5h:-- | 7d:--");
}

#[test]
fn line2_drops_bars_on_narrow_panes() {
    let agents = [agent("smith", "anthropic/claude-opus-4-8")];
    let lines = footer_lines(&fc(&agents, &ctx(&[("smith", 100_000)])), 40);
    assert_eq!(text(&lines[1]), "5h:3h52m | 7d:3d23h");
}

#[test]
fn line3_swarm_by_default_relay_when_mentioning() {
    let agents = [agent("coder", "m")];
    let f = fc(&agents, &HashMap::new());
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert_eq!(
        l3,
        "\u{25b6}\u{25b6} swarm mode \u{00b7} / for constructs \u{00b7} @ to relay to an agent"
    );
    let mut f = fc(&agents, &HashMap::new());
    f.input = "@coder fix the tests";
    let l3 = text(&footer_lines(&f, 120)[2]);
    assert!(l3.starts_with("\u{25b6}\u{25b6} @coder relay"), "{l3}");
}

#[test]
fn segments_are_colored_separators_muted() {
    let agents = [agent("smith", "qwen3-coder-plus")];
    let lines = footer_lines(&fc(&agents, &ctx(&[])), 120);
    let th = crew_theme::theme();
    // First char of line 1 is the model segment → cyan (ansi[14]).
    assert_eq!(lines[0][0].1, th.ansi[14]);
    // The separator chars are muted.
    let sep = lines[0].iter().find(|(c, _)| *c == '|').unwrap();
    assert_eq!(sep.1, th.text_muted);
}

#[test]
fn mixed_roster_counts_models() {
    let agents = [agent("a", "m1"), agent("b", "m2")];
    let lines = footer_lines(&fc(&agents, &HashMap::new()), 120);
    assert!(text(&lines[0]).starts_with("2 agents | "));
}
```

Run: `cargo test -p crew-app chatsummary` — expected: FAIL.

- [ ] **Step 2: Expose the relay-target helper**

In `chatinput.rs`, refactor `mention_len` (line 77) so the footer can reuse the exact routing rule:

```rust
/// The rostered agent a leading `@mention` addresses, if any — the same rule
/// `mention_len` colours by, shared with the footer's mode line.
pub(crate) fn relay_target<'a>(input: &'a str, agents: &[AgentInfo]) -> Option<&'a str> {
    let rest = input.strip_prefix('@')?;
    let name = rest.split_whitespace().next().unwrap_or("");
    agents
        .iter()
        .any(|a| a.name.eq_ignore_ascii_case(name))
        .then_some(name)
}

fn mention_len(input: &str, agents: &[AgentInfo]) -> usize {
    relay_target(input, agents).map_or(0, |n| 1 + n.len())
}
```

(`use crew_plugin::AgentInfo;` is already imported there for `mention_len`.)

- [ ] **Step 3: Rewrite `chatsummary.rs`**

Replace `summary_text`/`summary_block`/`block_lines` with the colored builders. Keep `short_model`, `MIN_ROWS`, and the module doc updated to describe the 3-line footer. New `MAX_BLOCK` is 3.

```rust
type Fg = (u8, u8, u8);
type Seg = (String, Fg);

pub(crate) struct FooterCtx<'a> {
    pub agents: &'a [AgentInfo],
    pub ctx: &'a HashMap<String, u64>,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
    pub branch: Option<&'a str>,
    /// The composer's current text, for the live routing-mode line.
    pub input: &'a str,
    pub windows: crate::usageledger::Windows,
}

/// `$0.129` under $10, `$12.35` above — micro-USD in, display string out.
fn fmt_cost(microusd: u64) -> String {
    let d = microusd as f64 / 1_000_000.0;
    if d < 10.0 {
        format!("${d:.3}")
    } else {
        format!("${d:.2}")
    }
}

/// `3h52m` under a day, `3d23h` from one up — window countdowns.
fn fmt_left(ms: u64) -> String {
    let mins = ms / 60_000;
    let (d, h, m) = (mins / 1_440, (mins % 1_440) / 60, mins % 60);
    if d > 0 {
        format!("{d}d{h}h")
    } else {
        format!("{h}h{m:02}m")
    }
}

/// An 8-cell dithered meter: `▓` filled, `░` empty. 1-99% always shows at
/// least one of each so "almost empty" and "almost full" stay legible.
fn bar(pct: u8) -> String {
    const W: usize = 8;
    let filled = (usize::from(pct.min(100)) * W + 50) / 100;
    let filled = match pct {
        0 => 0,
        1..=99 => filled.clamp(1, W - 1),
        _ => W,
    };
    "\u{2593}".repeat(filled) + &"\u{2591}".repeat(W - filled)
}

/// Join colored segments with a muted ` | `, then explode to per-char cells.
fn join(segs: &[Seg]) -> Vec<(char, Fg)> {
    let muted = crew_theme::theme().text_muted;
    let mut out = Vec::new();
    for (i, (s, fg)) in segs.iter().enumerate() {
        if i > 0 {
            out.extend(" | ".chars().map(|c| (c, muted)));
        }
        out.extend(s.chars().map(|c| (c, *fg)));
    }
    out
}

/// The tightest remaining context across agents with a known window, as a
/// fill percentage — the agent nearest its ceiling is the one that matters.
fn ctx_fill(agents: &[AgentInfo], ctx: &HashMap<String, u64>) -> Option<u8> {
    let mut max_fill: Option<u8> = None;
    for a in agents {
        let Some(limit) = crate::ctxlimit::context_limit(&a.model).filter(|&l| l > 0) else {
            continue;
        };
        let used = ctx.get(&a.name).copied().unwrap_or(0);
        let fill = ((used.saturating_mul(100)) / limit).min(100) as u8;
        max_fill = Some(max_fill.map_or(fill, |m| m.max(fill)));
    }
    max_fill
}

/// The Claude-Code-style statusline: up to three colored lines (identity &
/// spend / rolling windows & bars / routing mode & hints). Pure — everything
/// it shows arrives via `FooterCtx`, so it unit-tests without a live pane.
pub(crate) fn footer_lines(fc: &FooterCtx, cols: usize) -> Vec<Vec<(char, Fg)>> {
    let th = crew_theme::theme();
    let (cyan, blue, green, magenta, yellow) =
        (th.ansi[14], th.ansi[12], th.ansi[10], th.ansi[13], th.ansi[11]);
    let muted = th.text_muted;

    // Line 1: model | branch | $cost | in/out.
    let mut l1: Vec<Seg> = Vec::new();
    let mut models: Vec<&str> = Vec::new();
    for a in fc.agents {
        let m = short_model(&a.model);
        if !models.contains(&m) {
            models.push(m);
        }
    }
    match models.as_slice() {
        [] => {}
        [one] => l1.push(((*one).to_string(), cyan)),
        many => l1.push((format!("{} agents", many.len()), cyan)),
    }
    if let Some(b) = fc.branch {
        l1.push((b.to_string(), yellow));
    }
    if fc.cost_microusd > 0 {
        l1.push((fmt_cost(fc.cost_microusd), green));
    }
    l1.push((
        format!(
            "{} in / {} out",
            fmt_tokens(fc.tok_in),
            fmt_tokens(fc.tok_out)
        ),
        magenta,
    ));

    // Line 2: 5h/7d countdowns, then budget + context bars (bars are the
    // first thing to go on a narrow pane).
    let mut l2: Vec<Seg> = Vec::new();
    let left = |w: Option<crate::usageledger::WindowStat>| {
        w.map_or("--".to_string(), |w| fmt_left(w.left_ms))
    };
    l2.push((format!("5h:{}", left(fc.windows.five_h)), blue));
    l2.push((format!("7d:{}", left(fc.windows.seven_d)), blue));
    if cols >= 60 {
        if let Some(w) = fc.windows.five_h {
            let pct = ((w.spent.saturating_mul(100)) / w.budget.max(1)).min(100) as u8;
            l2.push((format!("{} {pct}% (5h)", bar(pct)), muted));
        }
        if let Some(fill) = ctx_fill(fc.agents, fc.ctx) {
            l2.push((format!("{} {fill}% (ctx)", bar(fill)), muted));
        }
    }

    // Line 3: live routing mode + the hints that used to crowd the composer.
    let mode = match crate::chatinput::relay_target(fc.input, fc.agents) {
        Some(name) => format!("\u{25b6}\u{25b6} @{name} relay"),
        None => "\u{25b6}\u{25b6} swarm mode".to_string(),
    };
    let hints = " \u{00b7} / for constructs \u{00b7} @ to relay to an agent";
    let mut l3: Vec<(char, Fg)> = mode.chars().map(|c| (c, yellow)).collect();
    l3.extend(hints.chars().map(|c| (c, muted)));

    vec![join(&l1), join(&l2), l3]
}
```

Then rewire the existing entry points (keep signatures; `MAX_BLOCK: u16 = 3`):

```rust
fn footer_ctx<'a>(pane: &'a ChatPane, now_ms: u64) -> FooterCtx<'a> {
    FooterCtx {
        agents: &pane.agents,
        ctx: &pane.ctx,
        tok_in: pane.tok_in,
        tok_out: pane.tok_out,
        cost_microusd: pane.cost_microusd,
        branch: pane.git_branch.as_deref(),
        input: &pane.input,
        windows: crate::usageledger::windows(now_ms),
    }
}

pub(crate) fn summary_rows(pane: &ChatPane, cols: u16, rows: u16) -> u16 {
    if rows < MIN_ROWS || cols < 6 {
        return 0;
    }
    let _ = pane;
    // Always 3 lines when the budget allows — line 1 alone otherwise. The
    // `rows - (MIN_ROWS-1)` budget keeps the composer's bordered threshold.
    let budget = rows - (MIN_ROWS - 1);
    budget.min(MAX_BLOCK)
}

pub(crate) fn summary_cells(pane: &ChatPane, cols: u16, top: u16, height: u16) -> Vec<CellView> {
    if height == 0 {
        return Vec::new();
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let lines = footer_lines(&footer_ctx(pane, now_ms), cols as usize);
    let bg = crew_theme::theme().page_bg;
    let mut cells = Vec::new();
    for (i, line) in lines.into_iter().take(height as usize).enumerate() {
        let row = top + i as u16;
        crate::chatwidth::place_row(1, cols, line.into_iter(), |x, c, fg| {
            cells.push(CellView {
                col: x,
                row,
                c,
                fg,
                bg,
                bold: false,
                italic: false,
            });
        });
    }
    cells
}
```

Update imports (`fmt_tokens` already imported; drop unused `AgentInfo` uses if any). Delete `summary_text`, `summary_block`, `block_lines` and any imports they alone used.

- [ ] **Step 4: Trim the composer placeholder**

In `chatinput.rs` (the hints now live on footer line 3):

```rust
const PLACEHOLDER_HINT: &str = "type a task";
```

Fix any chatinput tests that assert the old string (`grep -rn "for constructs" crates/crew-app/src`).

- [ ] **Step 5: Test and commit**

Run: `cargo test -p crew-app`
Expected: all pass (8 new chatsummary tests; no other suite references the deleted builders — fix any that do by switching them to `footer_lines`).

```bash
git add crates/crew-app/src
git commit -m "feat(app): Claude-style 3-line colored statusline footer"
```

---

### Task 7: `├`/`└` tree connectors for chained task replies

**Files:**
- Modify: `crates/crew-app/src/chatmsgs.rs` (`header_line` line 80, `card_lines` line 157)
- Test: `crates/crew-app/src/chatmsgs_tests.rs`

**Interfaces:**
- Consumes: nothing from other tasks (independent of the footer work).
- Produces: `header_line(m: &Message, now_ms: u64, connector: Option<char>)` (was `chained: bool`); chained cards render `├ ` when another card of the same task follows, `└ ` on the last — Claude-Code background-agent tree style.

- [ ] **Step 1: Failing test**

In `chatmsgs_tests.rs`, next to the existing chaining test (mirror its message-builder helpers — they set `meta` task tags):

```rust
#[test]
fn middle_chained_cards_get_tee_last_gets_corner() {
    // Three replies on task #7: root keeps its gutter, the middle chained
    // card connects with ├, the final one closes with └.
    let msgs = [
        msg_with_meta("coder", "root", "task:7"),
        msg_with_meta("coder", "mid", "task:7"),
        msg_with_meta("coder", "last", "task:7"),
    ];
    let lines = card_lines(&msgs, 80, 0, View::default());
    let texts: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect();
    let headers: Vec<&String> = texts.iter().filter(|t| t.contains("coder")).collect();
    assert!(headers[1].starts_with("\u{251c} "), "{:?}", headers[1]);
    assert!(headers[2].starts_with("\u{2514} "), "{:?}", headers[2]);
}
```

Use the file's actual helper names/`View` construction — read the top of `chatmsgs_tests.rs` first and copy its patterns exactly (e.g. if messages are built via a `msg(...)` helper plus a meta field, follow that; `msg_with_meta` above is the intent, not a required name).

Run: `cargo test -p crew-app chatmsgs` — expected: FAIL (middle card currently draws `└`).

- [ ] **Step 2: Implement look-ahead**

In `card_lines` (~line 168):

```rust
        let tid = crate::chattime::task_tag(&m.meta);
        let chained =
            tid.is_some() && i > 0 && tid == crate::chattime::task_tag(&messages[i - 1].meta);
        let continues = tid.is_some()
            && messages
                .get(i + 1)
                .is_some_and(|n| tid == crate::chattime::task_tag(&n.meta));
        // ├ while more replies of this task follow, └ on the last — the
        // Claude-Code tree look, so a task's replies read as one thread.
        let connector = chained.then(|| if continues { '\u{251c}' } else { '\u{2514}' });
        if i > 0 && !chained {
            out.push(Vec::new()); // spacer between unrelated cards
        }
```

and pass it through (`header_line(m, now_ms, connector)`). In `header_line`, replace the `chained: bool` parameter with `connector: Option<char>` and the branch at line 84:

```rust
    if let Some(conn) = connector {
        line.extend(format!("{conn} ").chars().map(|c| plain(c, muted, false)));
    } else {
        line.push(plain(gutter_for(&m.sender), sender_color(parts[0]), false));
        if let Some(id) = crate::chattime::task_tag(&m.meta) {
            line.extend(format!("#{id} ").chars().map(|c| plain(c, muted, false)));
        }
    }
```

Update `header_line`'s doc comment (`chained` → connector semantics).

- [ ] **Step 3: Test and commit**

Run: `cargo test -p crew-app chatmsgs`
Expected: all pass, including the existing two-card chain test (its single follow-up is last → still `└`).

```bash
git add crates/crew-app/src/chatmsgs.rs crates/crew-app/src/chatmsgs_tests.rs
git commit -m "feat(app): tee/corner tree connectors for multi-reply task chains"
```

---

### Task 8: Full verification + live GUI check

**Files:** none (verification only)

- [ ] **Step 1: Workspace test + lint sweep**

Run: `cargo test --workspace` — expected: all green.
Run: `cargo clippy --workspace -- -D warnings` — expected: clean (fix anything new).

- [ ] **Step 2: Live GUI verification**

Use the repo's `verify` skill (`.claude/skills/verify` recipe: isolated-HOME dev instance, frontmost-PID guard, screencapture) to:
1. Launch the dev build, open a `/smith` pane.
2. Screenshot the fresh pane — footer must show 3 lines (`0 in / 0 out`, `5h:-- | 7d:--`, `▶▶ swarm mode …`) in per-segment colors.
3. With `CREW_BROKER_MOCK_REPLY` set, send a message and screenshot again — mock reports no usage, so line 1 tokens stay 0 but the turn completes (regression check that nothing panics).
4. Type `@<agent> ` in the composer and screenshot — line 3 must flip to `▶▶ @<agent> relay`.

- [ ] **Step 3: Merge per branch flow**

Per the standing branch/merge flow: local `git merge --no-ff feat/smith-statusline-claude` into main, delete the branch. Offer push (this is not an enhance-loop release).
