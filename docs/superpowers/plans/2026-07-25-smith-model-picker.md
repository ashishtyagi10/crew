# /smith `/model` Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/model` in the agent smith composer (and in the app input bar) opens an opencode-style picker: models grouped by provider with display names, free/paid and per-Mtok price badges, and a serviceability mark for the active provider stack.

**Architecture:** A curated model catalog lands in `crew-hive` (data + types), enriched at runtime from the OpenRouter `/models` API on a worker thread. `crew-plugin` exposes its existing provider-discovery so the app can compute which route would serve a model. `crew-app` gains a `modelpick` row builder shared by both surfaces: the input bar's value picker (`suggest::options_for`) and a new `chatpalette::Kind::Model` arg-phase popup in the chat composer.

**Tech Stack:** Rust workspace; crates `crew-hive` (LLM domain), `crew-plugin` (broker), `crew-app` (winit GUI); `reqwest` + `tokio` (already deps of crew-hive); plain `cargo test`.

**Spec:** `docs/superpowers/specs/2026-07-25-smith-model-picker-design.md`

## Global Constraints

- Workspace root `/Users/atyagi/code/crew`; all paths below are relative to it.
- **No blocking I/O on the winit thread.** Network and `$SHELL` probes run on `std::thread::spawn` workers, results delivered by `std::sync::mpsc` or a `OnceLock`/`Mutex` global. This is a hard house rule — a blocking call there freezes every pane.
- **Never invent a price.** `price: None` renders `—`. `crew_hive::pricing` already refuses to guess (unknown → 0, footer hides `$`); the catalog follows the same doctrine.
- Prices are `(input, output)` in **µ$ per 1M tokens**, matching `pricing::RATES` (`$5/Mtok` → `5_000_000`).
- Composer fill is `/model all <slug>` — the broker's `model_cmd` reads `/model <agent> <model>`, so `/model <slug>` alone is a usage error.
- 200-line-per-file cap is a house convention; split rather than exceed it.
- Section order everywhere: `default` row, Recent, then Anthropic, OpenAI, Google, Alibaba, Moonshot, DeepSeek, Meta, Mistral, xAI, HuggingFace, OpenRouter, Other.
- Run `cargo fmt` before every commit; `cargo clippy --workspace --all-targets` must stay at 0 warnings.

---

### Task 1: Model catalog in `crew-hive`

**Files:**
- Create: `crates/crew-hive/src/catalog.rs`
- Create: `crates/crew-hive/src/catalog/data.rs`
- Modify: `crates/crew-hive/src/lib.rs` (add `pub mod catalog;` in the alphabetical list, after `pub mod bus;`)
- Modify: `crates/crew-hive/src/pricing.rs` (correct two stale Anthropic rates)
- Test: `#[cfg(test)] mod tests` at the bottom of `catalog.rs`

**Interfaces:**
- Produces: `crew_hive::catalog::{ModelInfo, Vendor, catalog}` — `catalog() -> &'static [ModelInfo]`; `ModelInfo { name, slug, or_slug, vendor, price, free, context }` with `price: Option<(u64, u64)>` in µ$/Mtok; `Vendor` is `Copy + PartialEq + Eq` with `Vendor::label(self) -> &'static str` and `Vendor::ORDER: &[Vendor]`.
- Consumes: nothing.

- [ ] **Step 1: Write the failing test**

Create `crates/crew-hive/src/catalog.rs` containing only the tests module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique_and_non_empty() {
        let mut seen: Vec<&str> = Vec::new();
        for m in catalog() {
            assert!(!m.slug.is_empty(), "empty slug for {}", m.name);
            assert!(!m.name.is_empty(), "empty name for {}", m.slug);
            assert!(!seen.contains(&m.slug), "duplicate slug {}", m.slug);
            seen.push(m.slug);
        }
    }

    #[test]
    fn free_rows_are_zero_priced_and_paid_rows_are_not() {
        for m in catalog() {
            if m.free {
                assert_eq!(m.price, Some((0, 0)), "free row {} must price at 0", m.slug);
            } else if let Some((inp, out)) = m.price {
                assert!(inp > 0 && out > 0, "paid row {} has a zero rate", m.slug);
            }
        }
    }

    #[test]
    fn the_majors_are_all_represented() {
        for v in [Vendor::Anthropic, Vendor::OpenAI, Vendor::Alibaba, Vendor::DeepSeek] {
            assert!(
                catalog().iter().any(|m| m.vendor == v),
                "no rows for {}",
                v.label()
            );
        }
    }

    #[test]
    fn anthropic_rates_match_the_pricing_table() {
        // The catalog badge and the statusline `$` must agree: a 1M-in call on
        // the catalog's price equals `pricing::cost_microusd` for the same slug.
        for m in catalog().iter().filter(|m| m.vendor == Vendor::Anthropic) {
            let (inp, _) = m.price.expect("Anthropic rows are all priced");
            assert_eq!(
                crate::pricing::cost_microusd(m.slug, 1_000_000, 0),
                inp,
                "catalog and pricing disagree on {}",
                m.slug
            );
        }
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-hive catalog`
Expected: FAIL to compile — `catalog` module not declared in `lib.rs`, `ModelInfo`/`Vendor`/`catalog` not found.

- [ ] **Step 3: Implement the types**

In `crates/crew-hive/src/lib.rs`, add `pub mod catalog;` to the alphabetical module list (between `pub mod bus;` and `pub mod govern;`).

Put this at the TOP of `crates/crew-hive/src/catalog.rs`, above the tests module:

```rust
//! The model catalog behind the `/model` picker: display names, slugs, vendor
//! grouping, list prices, and free/paid marking. Curated here rather than
//! discovered so the picker works offline; `fetch_openrouter` enriches it at
//! runtime where a live rate is available. Prices are µ$ per 1M tokens, the
//! same unit as [`crate::pricing`] — an unknown price is `None`, never a
//! guess (the badge renders `—`).
mod data;

/// The company behind a model — the picker's section key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vendor {
    Anthropic,
    OpenAI,
    Google,
    Alibaba,
    Moonshot,
    DeepSeek,
    Meta,
    Mistral,
    XAI,
    HuggingFace,
    OpenRouter,
    Other,
}

impl Vendor {
    /// Section header text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Alibaba => "alibaba \u{b7} qwen",
            Self::Moonshot => "moonshot \u{b7} kimi",
            Self::DeepSeek => "deepseek",
            Self::Meta => "meta \u{b7} llama",
            Self::Mistral => "mistral",
            Self::XAI => "xai",
            Self::HuggingFace => "hugging face",
            Self::OpenRouter => "openrouter",
            Self::Other => "other",
        }
    }
    /// Section order in the picker — majors first, meta-routers last.
    pub const ORDER: &'static [Vendor] = &[
        Self::Anthropic,
        Self::OpenAI,
        Self::Google,
        Self::Alibaba,
        Self::Moonshot,
        Self::DeepSeek,
        Self::Meta,
        Self::Mistral,
        Self::XAI,
        Self::HuggingFace,
        Self::OpenRouter,
        Self::Other,
    ];
}

/// One catalog row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    /// Display name shown to the user ("Claude Sonnet 5").
    pub name: &'static str,
    /// Native slug, sent when the provider serves this vendor directly.
    pub slug: &'static str,
    /// OpenRouter alias, sent when OpenRouter is the active provider.
    pub or_slug: Option<&'static str>,
    pub vendor: Vendor,
    /// (input, output) µ$ per 1M tokens; `None` when we don't know it.
    pub price: Option<(u64, u64)>,
    pub free: bool,
    /// Context window in tokens; 0 when unknown.
    pub context: u32,
}

/// The curated catalog.
pub fn catalog() -> &'static [ModelInfo] {
    data::MODELS
}
```

- [ ] **Step 4: Implement the data table**

Create `crates/crew-hive/src/catalog/data.rs`:

```rust
//! The curated catalog rows. Split from `catalog.rs` to keep both under the
//! line cap. Prices are µ$/Mtok list rates, 2026-07; `None` means we don't
//! have a verified rate (the picker badges it `—` and live enrichment may
//! fill it in). Free rows are OpenRouter `:free` variants.
use super::{ModelInfo, Vendor};

const fn m(
    name: &'static str,
    slug: &'static str,
    or_slug: Option<&'static str>,
    vendor: Vendor,
    price: Option<(u64, u64)>,
    free: bool,
    context: u32,
) -> ModelInfo {
    ModelInfo { name, slug, or_slug, vendor, price, free, context }
}

const M: u64 = 1_000_000; // µ$ per $1

pub(super) const MODELS: &[ModelInfo] = &[
    // Anthropic — rates verified against the 2026-07 first-party card.
    m("Claude Opus 5", "claude-opus-5", Some("anthropic/claude-opus-5"), Vendor::Anthropic, Some((5 * M, 25 * M)), false, 1_000_000),
    m("Claude Sonnet 5", "claude-sonnet-5", Some("anthropic/claude-sonnet-5"), Vendor::Anthropic, Some((3 * M, 15 * M)), false, 1_000_000),
    m("Claude Haiku 4.5", "claude-haiku-4-5", Some("anthropic/claude-haiku-4.5"), Vendor::Anthropic, Some((M, 5 * M)), false, 200_000),
    m("Claude Opus 4.8", "claude-opus-4-8", Some("anthropic/claude-opus-4.8"), Vendor::Anthropic, Some((5 * M, 25 * M)), false, 1_000_000),
    m("Claude Fable 5", "claude-fable-5", None, Vendor::Anthropic, Some((10 * M, 50 * M)), false, 1_000_000),
    // OpenAI — rates from `pricing::RATES`; GPT-5 list rate unverified.
    m("GPT-4.1", "gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI, Some((2 * M, 8 * M)), false, 0),
    m("GPT-4.1 Mini", "gpt-4.1-mini", Some("openai/gpt-4.1-mini"), Vendor::OpenAI, Some((400_000, 1_600_000)), false, 0),
    m("GPT-4o", "gpt-4o", Some("openai/gpt-4o"), Vendor::OpenAI, Some((2_500_000, 10 * M)), false, 0),
    m("GPT-4o Mini", "gpt-4o-mini", Some("openai/gpt-4o-mini"), Vendor::OpenAI, Some((150_000, 600_000)), false, 0),
    m("GPT-5", "gpt-5", Some("openai/gpt-5"), Vendor::OpenAI, None, false, 0),
    // Alibaba / DashScope — rates from `pricing::RATES`.
    m("Qwen Max", "qwen-max", Some("qwen/qwen-max"), Vendor::Alibaba, Some((1_600_000, 6_400_000)), false, 0),
    m("Qwen Plus", "qwen-plus", Some("qwen/qwen-plus"), Vendor::Alibaba, Some((400_000, 1_200_000)), false, 0),
    m("Qwen Turbo", "qwen-turbo", None, Vendor::Alibaba, Some((50_000, 200_000)), false, 0),
    m("Qwen3 Coder Plus", "qwen3-coder-plus", None, Vendor::Alibaba, Some((M, 5 * M)), false, 0),
    m("Qwen3 Coder Flash", "qwen3-coder-flash", None, Vendor::Alibaba, Some((300_000, 1_500_000)), false, 0),
    // DeepSeek / Moonshot — rates from `pricing::RATES`.
    m("DeepSeek Chat", "deepseek-chat", Some("deepseek/deepseek-chat"), Vendor::DeepSeek, Some((270_000, 1_100_000)), false, 0),
    m("DeepSeek Reasoner", "deepseek-reasoner", Some("deepseek/deepseek-r1"), Vendor::DeepSeek, Some((550_000, 2_190_000)), false, 0),
    m("Kimi K2", "kimi-k2", Some("moonshotai/kimi-k2"), Vendor::Moonshot, Some((600_000, 2_500_000)), false, 0),
    // Google — no verified first-party rate in-repo; enrichment fills these.
    m("Gemini 2.5 Pro", "gemini-2.5-pro", Some("google/gemini-2.5-pro"), Vendor::Google, None, false, 0),
    m("Gemini 2.5 Flash", "gemini-2.5-flash", Some("google/gemini-2.5-flash"), Vendor::Google, None, false, 0),
    // Free tier — the chain crew already ships in `broker/discover.rs`.
    m("Llama 3.3 70B", "meta-llama/llama-3.3-70b-instruct:free", Some("meta-llama/llama-3.3-70b-instruct:free"), Vendor::Meta, Some((0, 0)), true, 0),
    m("Llama 4 Scout", "meta-llama/llama-4-scout:free", Some("meta-llama/llama-4-scout:free"), Vendor::Meta, Some((0, 0)), true, 0),
    m("DeepSeek V3.1", "deepseek/deepseek-chat-v3.1:free", Some("deepseek/deepseek-chat-v3.1:free"), Vendor::DeepSeek, Some((0, 0)), true, 0),
    m("Qwen3 235B", "qwen/qwen3-235b-a22b:free", Some("qwen/qwen3-235b-a22b:free"), Vendor::Alibaba, Some((0, 0)), true, 0),
];
```

- [ ] **Step 5: Correct the two stale Anthropic rates**

In `crates/crew-hive/src/pricing.rs`, the `RATES` table lists Anthropic at 2025 prices. Update the two stale rows (leave `claude-sonnet` and `claude-haiku`, which are current):

```rust
    // Anthropic
    ("claude-opus", 5_000_000, 25_000_000),
    ("claude-sonnet", 3_000_000, 15_000_000),
    ("claude-haiku", 1_000_000, 5_000_000),
    ("claude-fable", 10_000_000, 50_000_000),
```

Also update the doc comment's date marker on `RATES` from `2026-07` if it names a narrower month; leave the wording otherwise.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-hive catalog pricing`
Expected: PASS — all four catalog tests plus the existing pricing tests.

If `anthropic_rates_match_the_pricing_table` fails, the catalog and `RATES` disagree; fix `RATES`, not the test — the badge and the statusline `$` must show the same number.

- [ ] **Step 7: Verify the OpenRouter aliases**

The `or_slug` values are the routing identifiers OpenRouter expects; a wrong one fails silently at the first turn. Verify them against the live list (needs `OPENROUTER_API_KEY`; skip and say so in the report if unset):

```bash
curl -s https://openrouter.ai/api/v1/models | python3 -c '
import json,sys
ids={m["id"] for m in json.load(sys.stdin)["data"]}
want=["anthropic/claude-opus-5","anthropic/claude-sonnet-5","anthropic/claude-haiku-4.5","anthropic/claude-opus-4.8","openai/gpt-4.1","openai/gpt-4.1-mini","openai/gpt-4o","openai/gpt-4o-mini","openai/gpt-5","qwen/qwen-max","qwen/qwen-plus","deepseek/deepseek-chat","deepseek/deepseek-r1","moonshotai/kimi-k2","google/gemini-2.5-pro","google/gemini-2.5-flash","meta-llama/llama-3.3-70b-instruct:free","meta-llama/llama-4-scout:free","deepseek/deepseek-chat-v3.1:free","qwen/qwen3-235b-a22b:free"]
for w in want: print(("OK  " if w in ids else "MISS"), w)'
```

For each `MISS`: if the model exists under a different id, correct the `or_slug`; if it does not exist, set `or_slug: None` (the row still works on its native provider). **Do not leave an unverified alias in the table.** Record what you changed in the task report.

- [ ] **Step 8: Commit**

```bash
cargo fmt
git add crates/crew-hive/src/catalog.rs crates/crew-hive/src/catalog/data.rs crates/crew-hive/src/lib.rs crates/crew-hive/src/pricing.rs
git commit -m "feat(hive): model catalog (vendors, display names, prices) + fix stale Anthropic rates"
```

---

### Task 2: Serviceability — which provider would serve a model

**Files:**
- Modify: `crates/crew-plugin/src/broker/discover.rs` (make `ProviderKind` + `pick_provider` public)
- Modify: `crates/crew-plugin/src/broker/mod.rs` (re-export)
- Modify: `crates/crew-plugin/src/lib.rs` (re-export)
- Create: `crates/crew-app/src/modelkeys.rs` (login-shell key probe)
- Create: `crates/crew-app/src/modelroute.rs` (route resolution)
- Modify: `crates/crew-app/src/lib.rs` or `main.rs` module list — whichever declares `mod cmdcheck;` (add `mod modelkeys;` and `mod modelroute;` alphabetically)
- Modify: wherever `cmdcheck::init_shell_path()` is called at startup (add `modelkeys::init_probe()` beside it)
- Test: `#[cfg(test)] mod tests` at the bottom of `modelroute.rs`

**Interfaces:**
- Consumes: `crew_hive::catalog::{ModelInfo, Vendor}` (Task 1).
- Produces: `crate::modelroute::{Route, route_for}` — `route_for(m: &ModelInfo, provider: Option<Provider>, probed: bool) -> Route`, and `Route::fill_slug(&self, m: &ModelInfo) -> String`; `crate::modelkeys::{init_probe, provider_now}` where `provider_now() -> (Option<Provider>, bool)` returns the active provider and whether the shell probe has completed.
- Produces: `crew_plugin::{Provider, active_provider}` — `active_provider(force: Option<&str>, has_key: impl Fn(&str) -> bool) -> Option<Provider>`.

- [ ] **Step 1: Write the failing test**

Create `crates/crew-app/src/modelroute.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crew_hive::catalog::{ModelInfo, Vendor};

    fn row(slug: &'static str, or_slug: Option<&'static str>, vendor: Vendor) -> ModelInfo {
        ModelInfo { name: "n", slug, or_slug, vendor, price: None, free: false, context: 0 }
    }

    #[test]
    fn direct_routes_match_the_active_provider() {
        let claude = row("claude-sonnet-5", Some("anthropic/claude-sonnet-5"), Vendor::Anthropic);
        let qwen = row("qwen-max", Some("qwen/qwen-max"), Vendor::Alibaba);
        assert_eq!(route_for(&claude, Some(Provider::Anthropic), true), Route::Direct("anthropic"));
        assert_eq!(route_for(&qwen, Some(Provider::DashScope), true), Route::Direct("dashscope"));
    }

    #[test]
    fn openrouter_serves_anything_with_an_alias() {
        let claude = row("claude-sonnet-5", Some("anthropic/claude-sonnet-5"), Vendor::Anthropic);
        let native_only = row("qwen-turbo", None, Vendor::Alibaba);
        assert_eq!(route_for(&claude, Some(Provider::OpenRouter), true), Route::ViaOpenRouter);
        // No alias and OpenRouter can't reach the native endpoint.
        assert!(matches!(
            route_for(&native_only, Some(Provider::OpenRouter), true),
            Route::Missing(_)
        ));
    }

    #[test]
    fn missing_names_the_key_the_user_would_have_to_set() {
        let gpt = row("gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI);
        assert_eq!(
            route_for(&gpt, Some(Provider::Anthropic), true),
            Route::Missing("OPENROUTER_API_KEY")
        );
    }

    #[test]
    fn unknown_until_the_probe_lands_and_mock_serves_everything() {
        let gpt = row("gpt-4.1", Some("openai/gpt-4.1"), Vendor::OpenAI);
        // Probe not finished: never claim a key is missing on evidence we lack.
        assert_eq!(route_for(&gpt, Some(Provider::Anthropic), false), Route::Unknown);
        assert_eq!(route_for(&gpt, None, false), Route::Unknown);
        assert_eq!(route_for(&gpt, Some(Provider::Mock), true), Route::Mock);
    }

    #[test]
    fn fill_slug_follows_the_route() {
        let claude = row("claude-sonnet-5", Some("anthropic/claude-sonnet-5"), Vendor::Anthropic);
        assert_eq!(Route::ViaOpenRouter.fill_slug(&claude), "anthropic/claude-sonnet-5");
        assert_eq!(Route::Direct("anthropic").fill_slug(&claude), "claude-sonnet-5");
        assert_eq!(Route::Unknown.fill_slug(&claude), "claude-sonnet-5");
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app modelroute`
Expected: FAIL to compile — `modelroute` not declared, `Route`/`route_for`/`Provider` not found.

- [ ] **Step 3: Expose provider discovery from `crew-plugin`**

In `crates/crew-plugin/src/broker/discover.rs`, widen the two items (bodies unchanged):

```rust
/// The provider backing the project's API-backed agents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProviderKind {
    Mock,
    DashScope,
    OpenRouter,
    Anthropic,
}
```

and

```rust
pub fn pick_provider(
    force: Option<&str>,
    has_key: impl Fn(&str) -> bool,
) -> Option<ProviderKind> {
```

In `crates/crew-plugin/src/broker/mod.rs`, alongside the existing `pub use` lines add:

```rust
pub use discover::{pick_provider as active_provider, ProviderKind as Provider};
```

In `crates/crew-plugin/src/lib.rs`, extend the `pub use broker::{...}` list with `active_provider` and `Provider` (keep the list alphabetical).

- [ ] **Step 4: Implement the route resolver**

Put this at the top of `crates/crew-app/src/modelroute.rs`, above the tests:

```rust
//! Which provider would actually serve a catalog model, given the stack the
//! broker will discover. The broker picks exactly ONE provider for every API
//! agent (`crew_plugin::active_provider`), so a pick is only serveable if that
//! provider can route it — OpenRouter reaches everything with an alias, the
//! direct providers only their own vendor. `Unknown` is the honest answer
//! until the key probe lands: we never claim a key is missing on evidence we
//! don't have.
use crew_hive::catalog::{ModelInfo, Vendor};
pub(crate) use crew_plugin::Provider;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// Served straight by the named provider ("anthropic", "dashscope").
    Direct(&'static str),
    /// Served through OpenRouter, using the row's `or_slug`.
    ViaOpenRouter,
    /// Mock provider (tests / `CREW_BROKER_MOCK_REPLY`): everything "works".
    Mock,
    /// The key probe hasn't finished — don't dim, don't promise.
    Unknown,
    /// Not serveable by the active stack; names the key that would fix it.
    Missing(&'static str),
}

impl Route {
    /// The slug to send: the OpenRouter alias when OpenRouter serves it, the
    /// native slug otherwise.
    pub(crate) fn fill_slug(&self, m: &ModelInfo) -> String {
        match self {
            Self::ViaOpenRouter => m.or_slug.unwrap_or(m.slug).to_string(),
            _ => m.slug.to_string(),
        }
    }
    /// Dim hint fragment for the row's desc column ("" when it adds nothing).
    pub(crate) fn hint(&self) -> String {
        match self {
            Self::Direct(p) => (*p).to_string(),
            Self::ViaOpenRouter => "via openrouter".to_string(),
            Self::Mock => "mock".to_string(),
            Self::Unknown => String::new(),
            Self::Missing(k) => format!("needs {k}"),
        }
    }
    /// Rows we know the stack can't serve render dim.
    pub(crate) fn unserveable(&self) -> bool {
        matches!(self, Self::Missing(_))
    }
}

/// Resolve the route for one catalog row. `probed` is whether the login-shell
/// key probe has completed; before it has, everything is `Unknown`.
pub(crate) fn route_for(m: &ModelInfo, provider: Option<Provider>, probed: bool) -> Route {
    let Some(provider) = provider else {
        return if probed {
            Route::Missing("ANTHROPIC_API_KEY")
        } else {
            Route::Unknown
        };
    };
    if !probed {
        return Route::Unknown;
    }
    match provider {
        Provider::Mock => Route::Mock,
        Provider::Anthropic if m.vendor == Vendor::Anthropic => Route::Direct("anthropic"),
        Provider::DashScope if m.vendor == Vendor::Alibaba => Route::Direct("dashscope"),
        Provider::OpenRouter if m.or_slug.is_some() => Route::ViaOpenRouter,
        Provider::OpenRouter => Route::Missing("a model OpenRouter serves"),
        _ => Route::Missing("OPENROUTER_API_KEY"),
    }
}
```

Note the `Provider::OpenRouter` arm without an alias returns `Missing` with a phrase, not a key name — `hint()` renders "needs a model OpenRouter serves", which is accurate.

- [ ] **Step 5: Implement the key probe**

Create `crates/crew-app/src/modelkeys.rs`:

```rust
//! Which provider the broker will pick, discovered the same way it does. The
//! broker hydrates missing provider keys from the login shell before deciding
//! (`crew-plugin`'s `broker/shellenv.rs`), so reading this process's env alone
//! would under-report on a Finder-launched app. Mirrors
//! [`crate::cmdcheck::init_shell_path`]: one bounded `$SHELL -ilc env` on a
//! background thread (NEVER the winit thread), cached in a `OnceLock`.
//! `CREW_SHELL_ENV=0` skips the probe, matching the broker's switch.
use std::collections::HashSet;
use std::sync::OnceLock;

/// Provider vars worth probing — the same set `shellenv::interesting` imports.
const KEYS: &[&str] = &[
    "DASHSCOPE_API_KEY",
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
    "CREW_PROVIDER",
    "CREW_BROKER_MOCK_REPLY",
];

/// Names seen non-empty in the login shell, once the probe lands.
static SHELL_KEYS: OnceLock<HashSet<String>> = OnceLock::new();

/// Kick off the probe. Call once at startup, beside `init_shell_path`.
pub(crate) fn init_probe() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        // No probe: fall back to this process's env immediately.
        let _ = SHELL_KEYS.set(process_keys());
        return;
    }
    std::thread::spawn(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut found = process_keys();
        if let Ok(out) = std::process::Command::new(&shell)
            .args(["-ilc", "env"])
            .output()
        {
            if let Ok(text) = String::from_utf8(out.stdout) {
                for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
                    if !v.is_empty() && KEYS.contains(&k) {
                        found.insert(k.to_string());
                    }
                }
            }
        }
        let _ = SHELL_KEYS.set(found);
    });
}

/// Keys already non-empty in this process.
fn process_keys() -> HashSet<String> {
    KEYS.iter()
        .filter(|k| std::env::var(k).is_ok_and(|v| !v.is_empty()))
        .map(|k| (*k).to_string())
        .collect()
}

/// The provider the broker would pick, and whether the probe has landed.
/// Before it lands the answer is `(None, false)` and every row reads
/// `Route::Unknown` — no row is dimmed on a guess.
pub(crate) fn provider_now() -> (Option<crew_plugin::Provider>, bool) {
    let Some(keys) = SHELL_KEYS.get() else {
        return (None, false);
    };
    let force = std::env::var("CREW_PROVIDER")
        .ok()
        .filter(|v| !v.is_empty());
    (
        crew_plugin::active_provider(force.as_deref(), |k| keys.contains(k)),
        true,
    )
}
```

Declare both modules in the crate's module list (the file that already declares `mod cmdcheck;`), and call `modelkeys::init_probe();` immediately after the existing `cmdcheck::init_shell_path();` call site.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-app modelroute && cargo test -p crew-plugin discover`
Expected: PASS — 5 new route tests; the existing `pick_provider` tests still pass through the widened signature.

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add crates/crew-plugin/src/broker/discover.rs crates/crew-plugin/src/broker/mod.rs crates/crew-plugin/src/lib.rs crates/crew-app/src/modelkeys.rs crates/crew-app/src/modelroute.rs
git add -u
git commit -m "feat(model): resolve which provider would serve a model (off-thread key probe)"
```

---

### Task 3: Non-selectable header rows

**Files:**
- Modify: `crates/crew-app/src/suggest.rs` (`MenuItem.header` + `step_sel` / `first_selectable`)
- Modify: `crates/crew-app/src/cmdmenu.rs` (render header rows)
- Modify: `crates/crew-app/src/chatpalette.rs` (2 `MenuItem` literals, `popup_key` nav)
- Modify: `crates/crew-app/src/route.rs` (1 `MenuItem` literal, line ~59)
- Modify: `crates/crew-app/src/render.rs` (1 `MenuItem` literal, line ~157)
- Modify: `crates/crew-app/src/inputkeys.rs` (menu Up/Down nav, ~line 34)
- Test: `#[cfg(test)] mod tests` in `suggest_tests.rs` and `cmdmenu.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `MenuItem { label, desc, fill, submit, header }` — `header: true` rows are section titles: never selected, no `›` marker, dim+bold. `suggest::step_sel(items: &[MenuItem], sel: usize, down: bool) -> usize` and `suggest::first_selectable(items: &[MenuItem]) -> usize`.

- [ ] **Step 1: Write the failing test**

In `crates/crew-app/src/suggest_tests.rs`, add:

```rust
#[test]
fn step_sel_skips_header_rows_in_both_directions() {
    use crate::suggest::{first_selectable, step_sel, MenuItem};
    fn row(label: &str, header: bool) -> MenuItem {
        MenuItem {
            label: label.to_string(),
            desc: String::new(),
            fill: label.to_string(),
            submit: false,
            header,
        }
    }
    // [hdr, a, b, hdr, c]
    let items = vec![
        row("anthropic", true),
        row("a", false),
        row("b", false),
        row("openai", true),
        row("c", false),
    ];
    assert_eq!(first_selectable(&items), 1); // never lands on a header
    assert_eq!(step_sel(&items, 1, true), 2);
    assert_eq!(step_sel(&items, 2, true), 4); // hops the header at 3
    assert_eq!(step_sel(&items, 4, true), 1); // wraps past the leading header
    assert_eq!(step_sel(&items, 1, false), 4); // wraps backwards
    assert_eq!(step_sel(&items, 4, false), 2);
    // All-headers can't move anywhere: stay put rather than spin.
    let only = vec![row("anthropic", true)];
    assert_eq!(step_sel(&only, 0, true), 0);
    assert_eq!(first_selectable(&only), 0);
}
```

In `crates/crew-app/src/cmdmenu.rs` tests, add:

```rust
#[test]
fn header_rows_are_dim_and_unmarked() {
    let items = vec![
        crate::suggest::MenuItem {
            label: "anthropic".into(),
            desc: String::new(),
            fill: String::new(),
            submit: false,
            header: true,
        },
        crate::suggest::MenuItem {
            label: "Claude Sonnet 5".into(),
            desc: "claude-sonnet-5".into(),
            fill: "claude-sonnet-5".into(),
            submit: true,
            header: false,
        },
    ];
    // Selection sits on the model row (interior row 1 → card row 2).
    let cells = menu_card("models", &items, 1, 40, menu_rows(items.len()));
    assert!(cells.iter().any(|c| c.c == '\u{203a}' && c.row == 2));
    // The header row carries no marker and is not the accent colour.
    assert!(cells.iter().filter(|c| c.row == 1).all(|c| c.c != '\u{203a}'));
    assert!(cells
        .iter()
        .filter(|c| c.row == 1 && !c.c.is_whitespace())
        .all(|c| c.fg != accent_color()));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app step_sel_skips header_rows_are_dim`
Expected: FAIL to compile — `MenuItem` has no field `header`; `step_sel`/`first_selectable` not found.

- [ ] **Step 3: Add the field and the navigation helpers**

In `crates/crew-app/src/suggest.rs`, extend the struct:

```rust
pub(crate) struct MenuItem {
    /// Text shown in the row (command name, or value).
    pub label: String,
    /// Dim hint after the label.
    pub desc: String,
    /// Input text set when this row is accepted with Tab (or run on Enter when
    /// `submit`).
    pub fill: String,
    /// Enter **runs** `fill` when true; when false Enter just inserts `fill` and
    /// keeps the palette open — a command expanding into its value picker.
    pub submit: bool,
    /// A section title, not a choice: never selected, drawn dim without the
    /// selection marker. The picker groups rows by provider with these.
    pub header: bool,
}
```

and append these free functions:

```rust
/// The first row a selection may land on — headers are titles, not choices.
/// Falls back to 0 when every row is a header (nothing to select).
pub(crate) fn first_selectable(items: &[MenuItem]) -> usize {
    items.iter().position(|i| !i.header).unwrap_or(0)
}

/// Move the selection one row down (`down`) or up, wrapping, skipping header
/// rows. Returns `sel` unchanged when no row is selectable.
pub(crate) fn step_sel(items: &[MenuItem], sel: usize, down: bool) -> usize {
    if items.is_empty() || items.iter().all(|i| i.header) {
        return sel;
    }
    let n = items.len();
    let mut i = sel;
    for _ in 0..n {
        i = if down { (i + 1) % n } else { (i + n - 1) % n };
        if !items[i].header {
            return i;
        }
    }
    sel
}
```

Add `header: false` to both `MenuItem` literals already in `suggest.rs` (the value-picker map at ~line 107 and the command-row map at ~line 119).

- [ ] **Step 4: Update the other three literals and both navigators**

`crates/crew-app/src/route.rs` (~line 59), `crates/crew-app/src/render.rs` (~line 157), and both literals in `crates/crew-app/src/chatpalette.rs` (`slash_items` ~line 95, `attach_items` ~line 116): add `header: false` to each.

In `crates/crew-app/src/chatpalette.rs`, route `popup_key`'s arrows through the helper:

```rust
        ChatInput::Up => p.sel = crate::suggest::step_sel(&p.items, p.sel, false),
        ChatInput::Down => p.sel = crate::suggest::step_sel(&p.items, p.sel, true),
```

and in `after_edit`, replace the two places that seed or clamp `sel`:

```rust
    match palette {
        Some(p) if p.kind == kind => {
            p.sel = p.sel.min(items.len() - 1);
            if items[p.sel].header {
                p.sel = crate::suggest::first_selectable(&items);
            }
            p.items = items;
            p.entries = entries;
        }
        _ => {
            let sel = crate::suggest::first_selectable(&items);
            *palette = Some(PaletteState { kind, items, sel, entries })
        }
    }
```

In `crates/crew-app/src/inputkeys.rs` (~line 34), replace the modulo arithmetic:

```rust
                Key::Named(NamedKey::ArrowDown) => {
                    self.menu_sel = crate::suggest::step_sel(&menu, self.menu_sel, true);
                    return None;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.menu_sel = crate::suggest::step_sel(&menu, self.menu_sel, false);
                    return None;
                }
```

- [ ] **Step 5: Render header rows**

In `crates/crew-app/src/cmdmenu.rs`, `menu_cells` builds one `ListItem` per match. Branch on `header`:

```rust
    let items: Vec<ListItem> = matches
        .iter()
        .map(|c| {
            if c.header {
                // A section title, not a choice: dim + bold, no desc column.
                return ListItem::new(Line::from(Span::styled(
                    c.label.clone(),
                    Style::new().fg(DIM).add_modifier(Modifier::BOLD),
                )));
            }
            ListItem::new(Line::from(vec![
                Span::styled(c.label.clone(), Style::new().fg(accent_color())),
                Span::raw("  "),
                Span::styled(c.desc.clone(), Style::new().fg(DIM)),
            ]))
        })
        .collect();
```

The `highlight_symbol("› ")` still applies only to the selected row, and headers are never selected, so nothing else changes.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p crew-app suggest cmdmenu chatpalette`
Expected: PASS — both new tests plus every existing suggest/cmdmenu/chatpalette test (the added field is inert for non-header rows).

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add -u
git commit -m "feat(menu): non-selectable header rows + header-skipping navigation"
```

---

### Task 4: The picker rows (and the input-bar surface)

**Files:**
- Create: `crates/crew-app/src/modelpick.rs`
- Modify: `crates/crew-app/src/suggest.rs` (`options_for("/model")` → catalog rows)
- Modify: the crate module list (add `mod modelpick;`)
- Test: `#[cfg(test)] mod tests` at the bottom of `modelpick.rs`

**Interfaces:**
- Consumes: `crew_hive::catalog::{catalog, ModelInfo, Vendor}` (Task 1); `crate::modelroute::{route_for, Route}` + `crate::modelkeys::provider_now` (Task 2); `MenuItem { .., header }` (Task 3).
- Produces: `crate::modelpick::rows(query: &str, current: Option<&str>) -> Vec<MenuItem>` — the full picker: `default` row, vendor sections with headers, filtered and marked. Non-header rows carry `fill` = the slug to send (route-aware) and `submit: true`.

- [ ] **Step 1: Write the failing test**

Create `crates/crew-app/src/modelpick.rs` with only the tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn labels(q: &str) -> Vec<String> {
        rows(q, None).into_iter().map(|i| i.label).collect()
    }

    #[test]
    fn default_row_leads_and_sections_are_headed() {
        let r = rows("", None);
        assert_eq!(r[0].label, "default");
        assert!(!r[0].header);
        assert_eq!(r[0].fill, "default");
        // The first section header is Anthropic, and every header is inert.
        let first_header = r.iter().find(|i| i.header).expect("a section header");
        assert_eq!(first_header.label, "anthropic");
        assert!(r.iter().filter(|i| i.header).all(|i| i.fill.is_empty()));
    }

    #[test]
    fn every_header_has_at_least_one_row_under_it() {
        for q in ["", "claude", "free", "qwen"] {
            let r = rows(q, None);
            for (i, item) in r.iter().enumerate() {
                if item.header {
                    assert!(
                        r.get(i + 1).is_some_and(|next| !next.header),
                        "empty section {:?} for query {q:?}",
                        item.label
                    );
                }
            }
        }
    }

    #[test]
    fn query_matches_name_slug_vendor_and_free_badge() {
        assert!(labels("sonnet").iter().any(|l| l.contains("Sonnet")));
        assert!(labels("claude-opus-5").iter().any(|l| l.contains("Opus 5")));
        assert!(labels("anthropic").iter().any(|l| l.contains("Claude")));
        // "free" is a first-class filter term.
        let free = rows("free", None);
        assert!(!free.is_empty());
        assert!(free
            .iter()
            .filter(|i| !i.header && i.fill != "default")
            .all(|i| i.desc.contains("free")));
    }

    #[test]
    fn the_current_model_is_marked_once() {
        let r = rows("", Some("claude-sonnet-5"));
        let marked: Vec<&MenuItem> = r.iter().filter(|i| i.desc.contains('\u{25cf}')).collect();
        assert_eq!(marked.len(), 1);
        assert!(marked[0].label.contains("Sonnet 5"));
        // No current model → no mark anywhere.
        assert!(rows("", None).iter().all(|i| !i.desc.contains('\u{25cf}')));
    }

    #[test]
    fn priced_rows_badge_dollars_and_unpriced_rows_badge_a_dash() {
        let r = rows("claude-sonnet-5", Some("x"));
        let row = r.iter().find(|i| !i.header && i.fill != "default").unwrap();
        assert!(row.desc.contains("$3/$15"), "{}", row.desc);
        let g = rows("gemini-2.5-pro", None);
        let row = g.iter().find(|i| !i.header && i.fill != "default").unwrap();
        assert!(row.desc.contains('\u{2014}'), "{}", row.desc);
    }

    #[test]
    fn rows_submit_and_carry_a_slug() {
        for item in rows("", None).iter().filter(|i| !i.header) {
            assert!(item.submit, "{} should run on Enter", item.label);
            assert!(!item.fill.is_empty());
        }
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app modelpick`
Expected: FAIL to compile — `modelpick` module not declared, `rows` not found.

- [ ] **Step 3: Implement the row builder**

Put this above the tests in `crates/crew-app/src/modelpick.rs`:

```rust
//! Rows for the `/model` picker: the catalog grouped into provider sections,
//! filtered as the user types, each row badged with its price (or `—`), its
//! free/paid state, and how the active stack would serve it. Pure
//! string-in/rows-out — both surfaces (the input bar's value picker and the
//! composer's `Kind::Model` popup) render the same list.
use crew_hive::catalog::{catalog, ModelInfo, Vendor};

use crate::modelroute::{route_for, Route};
use crate::suggest::MenuItem;

/// Dollars-per-Mtok badge, or an em dash when the rate is unknown.
fn price_badge(m: &ModelInfo) -> String {
    if m.free {
        return "free".to_string();
    }
    match m.price {
        Some((inp, out)) => format!("${}/${}", dollars(inp), dollars(out)),
        None => "\u{2014}".to_string(),
    }
}

/// µ$/Mtok → a short dollar string ("3", "0.4", "1.6").
fn dollars(microusd: u64) -> String {
    let whole = microusd / 1_000_000;
    let tenths = (microusd % 1_000_000) / 100_000;
    let hundredths = (microusd % 100_000) / 10_000;
    match (tenths, hundredths) {
        (0, 0) => whole.to_string(),
        (_, 0) => format!("{whole}.{tenths}"),
        _ => format!("{whole}.{tenths}{hundredths}"),
    }
}

/// Context window as a short badge ("1M", "200k"); empty when unknown.
fn context_badge(tokens: u32) -> String {
    match tokens {
        0 => String::new(),
        t if t >= 1_000_000 => format!("{}M", t / 1_000_000),
        t => format!("{}k", t / 1000),
    }
}

/// Everything the query filters against: name, slug, alias, vendor, badges.
fn haystack(m: &ModelInfo) -> String {
    format!(
        "{} {} {} {} {}",
        m.name,
        m.slug,
        m.or_slug.unwrap_or(""),
        m.vendor.label(),
        if m.free { "free" } else { "paid" }
    )
    .to_lowercase()
}

fn matches(m: &ModelInfo, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let hay = haystack(m);
    hay.contains(query) || crate::suggest::is_subsequence(query, &hay)
}

/// The dim column: slug, price, context, current-mark, and route hint.
fn desc(m: &ModelInfo, route: Route, current: bool) -> String {
    let mut parts: Vec<String> = vec![m.slug.to_string(), price_badge(m)];
    let ctx = context_badge(m.context);
    if !ctx.is_empty() {
        parts.push(ctx);
    }
    let hint = route.hint();
    if !hint.is_empty() {
        parts.push(hint);
    }
    if current {
        parts.push("\u{25cf} current".to_string());
    }
    parts.join(" \u{b7} ")
}

/// The picker rows for `query`. `current` is the slug every agent is pinned to
/// (`None` when the roster disagrees or nothing is pinned) — it gets the `●`.
pub(crate) fn rows(query: &str, current: Option<&str>) -> Vec<MenuItem> {
    let q = query.trim().to_lowercase();
    let (provider, probed) = crate::modelkeys::provider_now();
    let mut out = Vec::new();
    if "default".starts_with(&q) {
        out.push(MenuItem {
            label: "default".to_string(),
            desc: "back to the provider default".to_string(),
            fill: "default".to_string(),
            submit: true,
            header: false,
        });
    }
    for vendor in Vendor::ORDER {
        let hits: Vec<&ModelInfo> = catalog()
            .iter()
            .filter(|m| m.vendor == *vendor && matches(m, &q))
            .collect();
        if hits.is_empty() {
            continue; // never emit an empty section
        }
        out.push(MenuItem {
            label: vendor.label().to_string(),
            desc: String::new(),
            fill: String::new(),
            submit: false,
            header: true,
        });
        for m in hits {
            let route = route_for(m, provider, probed);
            let is_current = current.is_some_and(|c| c == m.slug || Some(c) == m.or_slug);
            out.push(MenuItem {
                label: m.name.to_string(),
                desc: desc(m, route, is_current),
                fill: route.fill_slug(m),
                submit: true,
                header: false,
            });
        }
    }
    out
}
```

`is_subsequence` is already `pub(crate)` in `suggest.rs` (both `chatmention` and `chatcomplete` call it) — no change needed there.

- [ ] **Step 4: Wire the input-bar surface**

In `crates/crew-app/src/suggest.rs`, replace the whole hardcoded `"/model" => Some(vec![...])` arm of `options_for` with a delegation to the picker. Keep the surrounding comment accurate:

```rust
        // Model picker for the agent smith pane — the catalog grouped by
        // provider (see `modelpick`), applied to every agent (forwarded as
        // `/model all <slug>`). Any other slug still works: type it freeform
        // after `/model `. The value picker takes (value, desc) pairs, so the
        // section headers are re-derived by `menu_items` below.
        "/model" => Some(
            crate::modelpick::rows("", None)
                .into_iter()
                .filter(|i| !i.header)
                .map(|i| (i.fill, i.desc))
                .collect(),
        ),
```

The input bar's value picker filters by value prefix, so headers can't survive its `(String, String)` shape — this surface stays flat and un-headed; the composer popup (Task 5) is the grouped one. Note that in the code comment so a reader isn't surprised.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p crew-app modelpick suggest`
Expected: PASS — 6 new modelpick tests. `suggest_tests.rs` has a test asserting the `/model` command row exists; it should still pass (it checks the command, not its values). If any test pinned the old 7-slug list, update it to assert a catalog row instead (e.g. `qwen-max` is still offered) and say so in the report.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-app/src/modelpick.rs
git add -u
git commit -m "feat(model): grouped, priced, route-aware picker rows (input bar wired)"
```

---

### Task 5: The composer popup (`Kind::Model`)

**Files:**
- Modify: `crates/crew-app/src/chatpalette.rs` (`Kind::Model`, `pending_palette`, `after_edit`, `popup_key`, `accept`)
- Modify: `crates/crew-app/src/chat.rs` (`on_input` handles `PaletteKey::Submit`; pass the roster's model)
- Modify: `crates/crew-app/src/render.rs` (`palette_card_title`)
- Test: `chatpalette.rs` tests module; `crates/crew-app/src/chat_tests.rs`

**Interfaces:**
- Consumes: `crate::modelpick::rows` (Task 4); header-aware nav (Task 3).
- Produces: `PaletteKey::Submit` — the caller must run the input as if Enter had been pressed; `Kind::Model` rows fill `/model all <slug>`.

- [ ] **Step 1: Write the failing test**

In `crates/crew-app/src/chatpalette.rs`'s tests module, add:

```rust
#[test]
fn pending_palette_detects_the_model_arg_phase() {
    assert_eq!(pending_palette("/model "), Some((Kind::Model, "")));
    assert_eq!(pending_palette("/model son"), Some((Kind::Model, "son")));
    assert_eq!(pending_palette("/model  son"), Some((Kind::Model, "son")));
    // Two argument tokens = the freeform per-agent / explicit-all form.
    assert_eq!(pending_palette("/model all qwen-max"), None);
    assert_eq!(pending_palette("/model coder qwen"), None);
    // Still the plain slash palette before the space.
    assert_eq!(pending_palette("/model"), Some((Kind::Slash, "model")));
    // Other commands keep their old behaviour.
    assert_eq!(pending_palette("/theme dark"), None);
}

#[test]
fn model_rows_accept_into_a_full_broker_command_and_submit() {
    let mut p = None;
    after_edit(&mut p, "/model son", Vec::new);
    let open = p.as_ref().expect("model picker opens");
    assert_eq!(open.kind, Kind::Model);
    assert!(open.items.iter().any(|i| i.header)); // grouped
    assert!(!open.items[open.sel].header); // selection never starts on a header

    let mut input = "/model son".to_string();
    let key = popup_key(&mut p, &mut input, &ChatInput::Enter);
    assert!(matches!(key, PaletteKey::Submit));
    assert!(input.starts_with("/model all "), "{input}");
    assert!(p.is_none()); // accepting closes
}
```

In `crates/crew-app/src/chat_tests.rs`, add:

```rust
#[test]
fn model_pick_sends_one_broker_command() {
    let mut chat = test_pane(); // whatever this file's existing constructor is
    let cwd = std::path::Path::new(".");
    for c in "/model ".chars() {
        chat.on_input(crate::chatkeys::ChatInput::Char(c), cwd);
    }
    assert!(chat.palette.is_some(), "picker should be open");
    chat.on_input(crate::chatkeys::ChatInput::Enter, cwd);
    // The pick ran: input cleared, and the echoed message is a `/model all …`.
    assert!(chat.input.is_empty());
    let last = chat.messages.last().expect("the pick was echoed");
    assert!(last.text.starts_with("/model all "), "{}", last.text);
}
```

(Match `test_pane()` to whatever helper `chat_tests.rs` already uses to build a `ChatPane`; do not add a new one.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app chatpalette chat_tests`
Expected: FAIL to compile — `Kind::Model` and `PaletteKey::Submit` don't exist.

- [ ] **Step 3: Implement the palette side**

In `crates/crew-app/src/chatpalette.rs`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Slash,
    Agent,
    /// The `/model ` argument phase — the grouped model picker.
    Model,
}
```

```rust
pub(crate) enum PaletteKey {
    Consumed,
    /// The accepted row is a command to RUN: `input` now holds it, and the
    /// caller must submit as if Enter had been pressed on the composer.
    Submit,
    Forward,
}
```

`pending_palette` grows an arg-phase arm ahead of the whitespace bail-out:

```rust
pub(crate) fn pending_palette(input: &str) -> Option<(Kind, &str)> {
    // `/model <arg>` is the one command with a value picker in the composer:
    // one whitespace-free argument token opens it. A second token means the
    // freeform `/model <agent> <slug>` (or explicit `all`) form — leave it be.
    if let Some(rest) = input.strip_prefix("/model ") {
        let arg = rest.trim_start();
        return (!arg.contains(char::is_whitespace)).then_some((Kind::Model, arg));
    }
    if input.contains(char::is_whitespace) {
        return None;
    }
    if let Some(rest) = input.strip_prefix('/') {
        return Some((Kind::Slash, rest));
    }
    if let Some(rest) = input.strip_prefix('@') {
        return Some((Kind::Agent, rest.rsplit('+').next().unwrap_or(rest)));
    }
    None
}
```

In `after_edit`, the entries scan stays agent-only and the item build gains an arm:

```rust
    let entries = match palette {
        Some(p) if p.kind == kind => std::mem::take(&mut p.entries),
        _ => match kind {
            Kind::Agent => scan(),
            Kind::Slash | Kind::Model => Vec::new(),
        },
    };
    let items = match kind {
        Kind::Slash => slash_items(query),
        Kind::Agent => attach_items(query, &entries, input.contains('+')),
        Kind::Model => crate::modelpick::rows(query, None),
    };
```

`popup_key`'s accept arm returns `Submit` for submit rows:

```rust
        ChatInput::Complete | ChatInput::Enter => {
            let mut submit = false;
            if let Some(item) = p.items.get(p.sel) {
                submit = item.submit && matches!(key, ChatInput::Enter);
                *input = accept(input, p.kind, &item.fill);
            }
            *palette = None;
            if submit {
                return PaletteKey::Submit;
            }
        }
```

and `accept` learns the model shape:

```rust
pub(crate) fn accept(input: &str, kind: Kind, fill: &str) -> String {
    match kind {
        Kind::Slash => format!("{fill} "),
        // The broker reads `/model <agent> <slug>`; the picker applies the
        // pick to the whole roster, so it must send the `all` target.
        Kind::Model => format!("/model all {fill}"),
        Kind::Agent => match input.rfind('+') {
            Some(plus) => format!("{}{fill} ", &input[..=plus]),
            None => format!("@{fill} "),
        },
    }
}
```

- [ ] **Step 4: Run it from the composer**

In `crates/crew-app/src/chat.rs`, `on_input` currently discards anything that isn't `Consumed`. Replace the palette block:

```rust
        match crate::chatpalette::popup_key(&mut self.palette, &mut self.input, &k) {
            crate::chatpalette::PaletteKey::Consumed => return None,
            // A picked row is a command to run: the palette is closed and the
            // input holds it, so re-enter our own Enter path (no recursion —
            // `self.palette` is `None` now).
            crate::chatpalette::PaletteKey::Submit => {
                return self.on_input(ChatInput::Enter, cwd)
            }
            crate::chatpalette::PaletteKey::Forward => {}
        }
```

Pass the roster's current model into the picker so `●` lands. In `chat.rs`'s `after_edit` call site (the Char/Backspace branch), compute it first:

```rust
            let agents = self.agents.clone();
            crate::chatmention::after_edit(&mut self.mention, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
            crate::chatpalette::after_edit(&mut self.palette, &self.input, || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
```

`after_edit` has no place for the current model, so give the palette one: change its `Kind::Model` arm to read a field instead of `None`. Add to `PaletteState` construction path by passing the value through `after_edit`'s signature:

```rust
pub(crate) fn after_edit(
    palette: &mut Option<PaletteState>,
    input: &str,
    current_model: Option<&str>,
    scan: impl FnOnce() -> Vec<crate::chatmention::MentionEntry>,
) {
```

with the arm becoming `Kind::Model => crate::modelpick::rows(query, current_model)`, and the `chat.rs` call site passing:

```rust
            let current = crate::chatpalette::shared_model(&self.agents);
            crate::chatpalette::after_edit(&mut self.palette, &self.input, current.as_deref(), || {
                crate::chatmention::scan_entries(cwd, &agents)
            });
```

Add the helper to `chatpalette.rs`:

```rust
/// The model every agent runs, or `None` when the roster disagrees (mixed
/// pins) or reports nothing — only an unambiguous answer earns the `●` mark.
pub(crate) fn shared_model(agents: &[crew_plugin::AgentInfo]) -> Option<String> {
    let first = agents.iter().find(|a| !a.model.is_empty())?;
    agents
        .iter()
        .all(|a| a.model.is_empty() || a.model == first.model)
        .then(|| first.model.clone())
}
```

Update the existing `after_edit` calls in `chatpalette.rs`'s own tests to pass `None` for the new parameter.

In `crates/crew-app/src/render.rs`, extend the legend:

```rust
fn palette_card_title(kind: crate::chatpalette::Kind) -> &'static str {
    match kind {
        crate::chatpalette::Kind::Slash => "commands",
        crate::chatpalette::Kind::Agent => "attach",
        crate::chatpalette::Kind::Model => "models",
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p crew-app`
Expected: PASS — the whole crew-app suite, including the two new tests.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add -u
git commit -m "feat(smith): /model opens the grouped picker in the composer"
```

---

### Task 6: Recently-picked models

**Files:**
- Modify: `crates/crew-app/src/config.rs` (a `model_recents` field)
- Modify: `crates/crew-app/src/modelpick.rs` (a Recent section)
- Modify: `crates/crew-app/src/chat.rs` (`ChatPane.pending_recent`, set on a `/model all …` submit)
- Modify: `crates/crew-app/src/poll.rs` (drain it into the config)
- Test: `modelpick.rs` tests; `config_tests.rs`

**Interfaces:**
- Consumes: `modelpick::rows` (Task 4).
- Produces: `modelpick::{rows_with_recents, set_recents, MAX_RECENTS}` — a leading "recent" section fed by `CrewConfig.model_recents` (most-recent first, cap 5), published through a process-global so `rows()` keeps its Task 4 signature; `ChatPane.pending_recent: Option<String>` carries a pick to the poll.

- [ ] **Step 1: Write the failing test**

In `crates/crew-app/src/modelpick.rs` tests:

```rust
#[test]
fn recents_lead_the_list_and_dont_duplicate_a_section_row() {
    let r = rows_with_recents("", None, &["qwen-max".to_string()]);
    let header = r.iter().position(|i| i.header && i.label == "recent");
    let anthropic = r.iter().position(|i| i.header && i.label == "anthropic");
    assert!(header < anthropic, "recent must lead the sections");
    // The recent row still appears in its own vendor section (it's a shortcut,
    // not a move) — exactly two rows carry the slug.
    assert_eq!(r.iter().filter(|i| i.fill == "qwen-max").count(), 2);
    // An unknown slug in recents is skipped rather than rendered blank.
    let r = rows_with_recents("", None, &["ghost-model".to_string()]);
    assert!(!r.iter().any(|i| i.header && i.label == "recent"));
}
```

In `crates/crew-app/src/config_tests.rs`:

```rust
#[test]
fn model_recents_default_empty_and_round_trip() {
    let c: CrewConfig = toml::from_str("").unwrap();
    assert!(c.model_recents.is_empty()); // old config files still load
    let c = CrewConfig { model_recents: vec!["qwen-max".into()], ..c };
    let back: CrewConfig = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
    assert_eq!(back.model_recents, vec!["qwen-max".to_string()]);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-app recents_lead model_recents_default`
Expected: FAIL to compile — `rows_with_recents` and `CrewConfig.model_recents` don't exist.

- [ ] **Step 3: Implement**

In `crates/crew-app/src/config.rs`, add to `CrewConfig` (serde-default so existing files load unchanged):

```rust
    /// Recently picked models, most recent first (cap 5) — the `/model`
    /// picker's shortcut section. Slugs only; unknown ones are skipped.
    #[serde(default)]
    pub model_recents: Vec<String>,
```

In `crates/crew-app/src/modelpick.rs`, split `rows` so the recents list is injectable:

```rust
/// Cap on the recent section — beyond a handful it stops being a shortcut.
const MAX_RECENTS: usize = 5;

pub(crate) fn rows(query: &str, current: Option<&str>) -> Vec<MenuItem> {
    rows_with_recents(query, current, &[])
}

pub(crate) fn rows_with_recents(
    query: &str,
    current: Option<&str>,
    recents: &[String],
) -> Vec<MenuItem> {
    let q = query.trim().to_lowercase();
    let (provider, probed) = crate::modelkeys::provider_now();
    let mut out = Vec::new();
    if "default".starts_with(&q) {
        out.push(default_row());
    }
    let recent: Vec<&ModelInfo> = recents
        .iter()
        .take(MAX_RECENTS)
        .filter_map(|slug| catalog().iter().find(|m| m.slug == *slug))
        .filter(|m| matches(m, &q))
        .collect();
    if !recent.is_empty() {
        out.push(header_row("recent"));
        for m in recent {
            out.push(model_row(m, provider, probed, current));
        }
    }
    for vendor in Vendor::ORDER {
        let hits: Vec<&ModelInfo> = catalog()
            .iter()
            .filter(|m| m.vendor == *vendor && matches(m, &q))
            .collect();
        if hits.is_empty() {
            continue;
        }
        out.push(header_row(vendor.label()));
        for m in hits {
            out.push(model_row(m, provider, probed, current));
        }
    }
    out
}
```

Factor the three literal builders out of the existing body so both paths share them:

```rust
fn default_row() -> MenuItem {
    MenuItem {
        label: "default".to_string(),
        desc: "back to the provider default".to_string(),
        fill: "default".to_string(),
        submit: true,
        header: false,
    }
}

fn header_row(label: &str) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        desc: String::new(),
        fill: String::new(),
        submit: false,
        header: true,
    }
}

fn model_row(
    m: &ModelInfo,
    provider: Option<crate::modelroute::Provider>,
    probed: bool,
    current: Option<&str>,
) -> MenuItem {
    let route = route_for(m, provider, probed);
    let is_current = current.is_some_and(|c| c == m.slug || Some(c) == m.or_slug);
    MenuItem {
        label: m.name.to_string(),
        desc: desc(m, route, is_current),
        fill: route.fill_slug(m),
        submit: true,
        header: false,
    }
}
```

Thread the recents through `chatpalette::after_edit` the same way `current_model` was threaded in Task 5: add a `recents: &[String]` parameter, pass `&self.config.model_recents` equivalent from the chat pane's caller. The chat pane has no config handle — so instead have the app stash them in a process-global written at config load:

```rust
// modelpick.rs
static RECENTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Publish the persisted recents (called on config load and after each pick).
pub(crate) fn set_recents(list: Vec<String>) {
    if let Ok(mut g) = RECENTS.lock() {
        *g = list;
    }
}

fn recents_now() -> Vec<String> {
    RECENTS.lock().map(|g| g.clone()).unwrap_or_default()
}
```

and have `rows()` call `rows_with_recents(query, current, &recents_now())`. That keeps `chatpalette`'s signature at the Task 5 shape (no extra parameter) — prefer this over threading a second argument.

A `ChatAction` return would end the call and swallow the send, so the pick is
recorded on the pane and drained by the poll instead. Add the field to
`ChatPane` in `crates/crew-app/src/chat.rs` (the struct derives/constructs
alongside `palette`; initialise it to `None`):

```rust
    /// A `/model all <slug>` the app should add to the recents list. Set on
    /// submit, drained by the poll — the command itself still goes to the
    /// broker untouched, this is only the app-side note.
    pub(crate) pending_recent: Option<String>,
```

In `on_input`, immediately after the `/font` intercept and before the
`if !text.is_empty()` send block:

```rust
            if let Some(slug) = text.strip_prefix("/model all ") {
                let slug = slug.trim();
                if !slug.is_empty() && slug != "default" {
                    self.pending_recent = Some(slug.to_string());
                }
            }
```

In `crates/crew-app/src/poll.rs`, beside the existing per-pane drains, take the
value and fold it into the config:

```rust
        if let Some(slug) = chat.pending_recent.take() {
            let recents = &mut self.config.model_recents;
            recents.retain(|s| *s != slug);
            recents.insert(0, slug);
            recents.truncate(crate::modelpick::MAX_RECENTS);
            crate::modelpick::set_recents(recents.clone());
            self.save_config();
        }
```

(Use whatever the app's existing config-save helper is called; `save_config` is
a placeholder for it — grep for the call the `/theme` persist path already
makes and reuse that.) Make `MAX_RECENTS` `pub(crate)`.

At config load, publish the persisted list once: `crate::modelpick::set_recents(cfg.model_recents.clone());`

- [ ] **Step 4: Run the tests**

Run: `cargo test -p crew-app modelpick config`
Expected: PASS — both new tests plus the existing config round-trip tests.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add -u
git commit -m "feat(model): remember recently picked models"
```

---

### Task 7: Live OpenRouter enrichment

**Files:**
- Modify: `crates/crew-hive/src/catalog.rs` (add `fetch_openrouter` + `LiveModel`)
- Create: `crates/crew-hive/src/catalog/live.rs` (the HTTP call + parser)
- Create: `crates/crew-app/src/modelfetch.rs` (worker thread + disk cache)
- Modify: `crates/crew-app/src/modelpick.rs` (merge live rows)
- Modify: `crates/crew-app/src/poll.rs` (drain the worker's channel)
- Test: `live.rs` tests (fixture-driven); `modelpick.rs` merge test

**Interfaces:**
- Consumes: `ModelInfo`/`Vendor` (Task 1); the row builders (Tasks 4/6).
- Produces: `crew_hive::catalog::{LiveModel, fetch_openrouter, parse_models}` — `LiveModel { id: String, name: String, price: Option<(u64,u64)>, free: bool, context: u32 }`; `crate::modelfetch::spawn(key: String) -> Receiver<Vec<LiveModel>>`; `modelpick::set_live(Vec<LiveModel>)`.

- [ ] **Step 1: Write the failing test**

Create `crates/crew-hive/src/catalog/live.rs` with only its tests module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{"data":[
      {"id":"anthropic/claude-sonnet-5","name":"Anthropic: Claude Sonnet 5",
       "context_length":1000000,
       "pricing":{"prompt":"0.000003","completion":"0.000015"}},
      {"id":"meta-llama/llama-3.3-70b-instruct:free","name":"Meta: Llama 3.3 70B (free)",
       "context_length":131072,
       "pricing":{"prompt":"0","completion":"0"}},
      {"id":"weird/no-pricing","name":"No Pricing","context_length":0,
       "pricing":{"prompt":"","completion":""}}
    ]}"#;

    #[test]
    fn parses_per_token_strings_into_microusd_per_mtok() {
        let got = parse_models(FIXTURE).expect("fixture parses");
        let sonnet = got.iter().find(|m| m.id == "anthropic/claude-sonnet-5").unwrap();
        // $0.000003/token * 1M tokens = $3 = 3_000_000 µ$.
        assert_eq!(sonnet.price, Some((3_000_000, 15_000_000)));
        assert!(!sonnet.free);
        assert_eq!(sonnet.context, 1_000_000);
    }

    #[test]
    fn zero_price_is_free_and_unparseable_price_is_unknown() {
        let got = parse_models(FIXTURE).unwrap();
        let llama = got.iter().find(|m| m.id.ends_with(":free")).unwrap();
        assert!(llama.free);
        assert_eq!(llama.price, Some((0, 0)));
        let weird = got.iter().find(|m| m.id == "weird/no-pricing").unwrap();
        assert_eq!(weird.price, None); // never invent a number
        assert!(!weird.free);
    }

    #[test]
    fn malformed_json_is_an_error_not_a_panic() {
        assert!(parse_models("not json").is_err());
        assert!(parse_models("{}").is_err());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p crew-hive live`
Expected: FAIL to compile — `parse_models` not found, module not declared.

- [ ] **Step 3: Implement the parser and fetch**

At the top of `crates/crew-hive/src/catalog/live.rs`:

```rust
//! Live catalog enrichment from the OpenRouter `/models` API: real list
//! prices, context windows, and the current free tier. Parsing is split from
//! the request so it's testable without a network. Prices arrive as
//! USD-per-token decimal strings; an unparseable one stays `None` (the badge
//! renders `—`) rather than becoming a wrong number.
use super::LiveModel;

const ENDPOINT: &str = "https://openrouter.ai/api/v1/models";

/// USD-per-token string → µ$ per 1M tokens. `None` when it isn't a number.
fn per_mtok(raw: &str) -> Option<u64> {
    let usd: f64 = raw.trim().parse().ok()?;
    if !usd.is_finite() || usd < 0.0 {
        return None;
    }
    Some((usd * 1_000_000.0 * 1_000_000.0).round() as u64)
}

/// Parse a `/models` response body.
pub fn parse_models(body: &str) -> Result<Vec<LiveModel>, String> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "no `data` array".to_string())?;
    Ok(data
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_string();
            let name = m
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(&id)
                .to_string();
            let pricing = m.get("pricing");
            let inp = pricing
                .and_then(|p| p.get("prompt"))
                .and_then(|p| p.as_str())
                .and_then(per_mtok);
            let out = pricing
                .and_then(|p| p.get("completion"))
                .and_then(|p| p.as_str())
                .and_then(per_mtok);
            let price = inp.zip(out);
            let context = m
                .get("context_length")
                .and_then(|c| c.as_u64())
                .unwrap_or(0) as u32;
            Some(LiveModel {
                free: price == Some((0, 0)),
                id,
                name,
                price,
                context,
            })
        })
        .collect())
}

/// Fetch the live catalog. Bounded; any failure is the caller's cue to keep
/// using the static catalog.
pub async fn fetch(api_key: &str) -> Result<Vec<LiveModel>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let body = client
        .get(ENDPOINT)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    parse_models(&body)
}
```

In `crates/crew-hive/src/catalog.rs`, declare and re-export:

```rust
mod live;

pub use live::{fetch as fetch_openrouter, parse_models};

/// One row from the live OpenRouter catalog (owned, unlike [`ModelInfo`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveModel {
    pub id: String,
    pub name: String,
    pub price: Option<(u64, u64)>,
    pub free: bool,
    pub context: u32,
}
```

- [ ] **Step 4: Implement the app-side worker and cache**

Create `crates/crew-app/src/modelfetch.rs`:

```rust
//! Background enrichment of the model catalog. The fetch is an async HTTP call
//! in `crew-hive`; here it runs on a short-lived worker thread owning its own
//! current-thread tokio runtime (the `swarm::plan` pattern) and delivers over
//! an mpsc channel drained each frame — the winit thread never blocks. A disk
//! cache beside the config makes the second launch instant and keeps the
//! picker useful offline.
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, SystemTime};

use crew_hive::catalog::LiveModel;

/// How long a cached catalog stays fresh.
const TTL: Duration = Duration::from_secs(24 * 60 * 60);

fn cache_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("models-openrouter.json"))
}

/// Spawn the enrichment worker: cache first, network only when stale.
/// Returns immediately; `None` when there's nothing to do.
pub(crate) fn spawn() -> Option<Receiver<Vec<LiveModel>>> {
    let key = std::env::var("OPENROUTER_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Some(cached) = read_cache() {
            let _ = tx.send(cached);
            return;
        }
        let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        if let Ok(models) = rt.block_on(crew_hive::catalog::fetch_openrouter(&key)) {
            write_cache(&models);
            let _ = tx.send(models);
        }
    });
    Some(rx)
}

/// The cached catalog when it exists and is younger than [`TTL`].
fn read_cache() -> Option<Vec<LiveModel>> {
    let path = cache_path()?;
    let age = std::fs::metadata(&path).ok()?.modified().ok()?.elapsed().ok()?;
    if age > TTL {
        return None;
    }
    let raw = std::fs::read_to_string(&path).ok()?;
    crew_hive::catalog::parse_models(&raw).ok()
}

/// Best-effort cache write — a failure just means a fetch next launch.
fn write_cache(models: &[LiveModel]) {
    let Some(path) = cache_path() else { return };
    let body = serde_json::json!({
        "data": models.iter().map(|m| serde_json::json!({
            "id": m.id,
            "name": m.name,
            "context_length": m.context,
            "pricing": {
                "prompt": m.price.map_or(String::new(), |(i, _)| per_token(i)),
                "completion": m.price.map_or(String::new(), |(_, o)| per_token(o)),
            },
        })).collect::<Vec<_>>(),
    });
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, body.to_string());
    let _ = SystemTime::now(); // mtime is the freshness stamp
}

/// µ$/Mtok → the USD-per-token string shape `parse_models` reads back.
fn per_token(microusd_per_mtok: u64) -> String {
    format!("{:.12}", microusd_per_mtok as f64 / 1e12)
}
```

In `crates/crew-app/src/modelpick.rs`, add the live overlay:

```rust
static LIVE: std::sync::Mutex<Vec<crew_hive::catalog::LiveModel>> =
    std::sync::Mutex::new(Vec::new());

/// Publish enriched rows (called from the poll drain).
pub(crate) fn set_live(models: Vec<crew_hive::catalog::LiveModel>) {
    if let Ok(mut g) = LIVE.lock() {
        *g = models;
    }
}

/// Live price/context for a catalog row, matched on its OpenRouter alias.
fn enrich(m: &ModelInfo) -> (Option<(u64, u64)>, u32, bool) {
    let Some(alias) = m.or_slug else {
        return (m.price, m.context, m.free);
    };
    let Ok(live) = LIVE.lock() else {
        return (m.price, m.context, m.free);
    };
    match live.iter().find(|l| l.id == alias) {
        Some(l) => (
            l.price.or(m.price),
            if l.context > 0 { l.context } else { m.context },
            l.free || m.free,
        ),
        None => (m.price, m.context, m.free),
    }
}
```

Thread the enriched triple through the badge helpers so live prices reach the
row. The signatures become:

```rust
fn price_badge(price: Option<(u64, u64)>, free: bool) -> String
fn desc(m: &ModelInfo, price: Option<(u64, u64)>, free: bool, context: u32,
        route: Route, current: bool) -> String
```

and `model_row` opens with `let (price, context, free) = enrich(m);`, passing
those into `desc` instead of reading `m.price` / `m.context` / `m.free`.
`context_badge` already takes a `u32` — pass the enriched one. Keep the merge
additive: a live row never removes or replaces a curated one, it only fills in
`None` prices and zero context windows.

In `crates/crew-app/src/poll.rs`, hold `Option<Receiver<Vec<LiveModel>>>` on the app, kick `modelfetch::spawn()` the first time a model picker opens (a `bool` guard so it fires once per process), and drain with `try_recv()` into `modelpick::set_live`.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p crew-hive && cargo test -p crew-app`
Expected: PASS — both suites, including the three parser tests.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-hive/src/catalog/live.rs crates/crew-app/src/modelfetch.rs
git add -u
git commit -m "feat(model): live OpenRouter enrichment with a 24h disk cache"
```

---

## Final verification (after Task 7)

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets` → 0 warnings
- [ ] `cargo test --workspace` → green (note any pre-existing failures separately; `e2e_discovery` has been flaky in the past — confirm against `main` before blaming this branch)
- [ ] Whole-branch review, then merge to `main` (no-ff), then release by version bump + tag push (CI builds; the user installs via `/update` — **no local release build**)
- [ ] Manual GUI check to hand to the user: open `/smith`, type `/model `, confirm the grouped card with prices; pick a row; confirm the pane echoes `/model all <slug>` and the broker acknowledges.
