//! Roster discovery: which provider backs the project's stored specialists
//! (see [`super::specialists`]), and the final adapter list (API-backed
//! specialist agents + manifest plugin agents). Split from `registry` to keep
//! both under the line cap.
use std::sync::Arc;

use super::adapter::Adapter;
use super::apiadapter::specialist_agents;

/// Default OpenRouter fallback chain for the project's API-backed agents —
/// every catalog row marked `free`, in catalog order (currently Nvidia,
/// OpenAI, Google, Cohere: different upstream providers, so a
/// provider-specific throttle on one model rolls to the next instead of
/// failing the relay). Quality isn't the goal here. Derived from
/// `crew_hive::catalog` — the picker's free-tier badge and this fallback
/// chain must never drift apart, as they did when OpenRouter rotated all four
/// free slugs the same day. OpenRouter rotates its free models; override the
/// whole chain with a comma-separated `CREW_OPENROUTER_MODEL=slug1,slug2,…`
/// (a retired slug is skipped automatically when it errors).
pub(crate) fn default_openrouter_chain() -> Vec<String> {
    crew_hive::catalog::catalog()
        .iter()
        .filter(|m| m.free)
        .map(|m| {
            // Free rows are expected to carry OpenRouter-shaped model ids.
            // The fallback assumes `slug` is already OpenRouter-compatible.
            m.or_slug.unwrap_or(m.slug).to_string()
        })
        .collect()
}

/// Default Qwen chain for Alibaba Cloud DashScope (`DASHSCOPE_API_KEY`): the
/// most capable commercial alias first, rolling to cheaper tiers on limits.
/// Override with a comma-separated `CREW_DASHSCOPE_MODEL=slug1,slug2,…`.
pub(crate) const DEFAULT_DASHSCOPE_CHAIN: &[&str] = &["qwen-max", "qwen-plus", "qwen-turbo"];

/// DashScope's OpenAI-compatible chat endpoint (international). Point
/// `CREW_DASHSCOPE_BASE_URL` at the China-region host if your key lives there.
const DASHSCOPE_ENDPOINT: &str =
    "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions";

/// Parse a comma-separated model chain into an ordered list, falling back to
/// `default` when unset or empty.
pub(crate) fn parse_model_chain(env_val: Option<String>, default: Vec<String>) -> Vec<String> {
    let parsed: Vec<String> = env_val
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parsed.is_empty() {
        default
    } else {
        parsed
    }
}

/// A provider crew reaches over the OpenAI chat-completions wire, which is
/// most of them. Adding one is a row here: an endpoint, a key variable and a
/// default model chain. `OpenRouterProvider` already speaks this wire — it
/// has backed DashScope from the beginning with nothing but a different base
/// URL — so no client code is involved.
///
/// This exists because OpenRouter was the only multi-vendor route crew had,
/// which made `OPENROUTER_API_KEY` the answer for every OpenAI, Google or
/// Mistral row in the picker whether or not the user held a key for the
/// vendor itself.
pub struct DirectProvider {
    /// As `CREW_PROVIDER` and the credential store spell it.
    pub name: &'static str,
    pub var: &'static str,
    pub endpoint: &'static str,
    pub chain: &'static [&'static str],
    /// Comma-separated override, e.g. `CREW_OPENAI_MODEL=gpt-5,gpt-4.1`.
    pub chain_env: &'static str,
    pub base_url_env: &'static str,
    /// The catalog vendor this provider serves natively, so the model picker
    /// can route a row to it instead of to OpenRouter.
    pub vendor: crew_hive::catalog::Vendor,
}

/// Every OpenAI-wire provider crew knows, in discovery order.
///
/// Model ids are NOT invented here. Every slug in every chain is a native
/// (non-OpenRouter) slug that already exists in `crew_hive::catalog` for that
/// vendor, and `chains_are_native_catalog_slugs` enforces it — a default model
/// that 404s on first use is a worse first run than no provider at all. That
/// is also why xAI, Mistral and Groq are absent despite speaking this same
/// wire: the catalog carries no rows for them, so their ids would be guesses.
pub static DIRECT: &[DirectProvider] = &[
    DirectProvider {
        name: "openai",
        var: "OPENAI_API_KEY",
        endpoint: "https://api.openai.com/v1/chat/completions",
        chain: &["gpt-5", "gpt-4.1"],
        chain_env: "CREW_OPENAI_MODEL",
        base_url_env: "CREW_OPENAI_BASE_URL",
        vendor: crew_hive::catalog::Vendor::OpenAI,
    },
    DirectProvider {
        name: "gemini",
        var: "GEMINI_API_KEY",
        // Google's own OpenAI-compatibility endpoint, so the same client
        // works — no Google SDK, no second wire format.
        endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        chain: &["gemini-2.5-pro", "gemini-2.5-flash"],
        chain_env: "CREW_GEMINI_MODEL",
        base_url_env: "CREW_GEMINI_BASE_URL",
        vendor: crew_hive::catalog::Vendor::Google,
    },
    DirectProvider {
        name: "deepseek",
        var: "DEEPSEEK_API_KEY",
        endpoint: "https://api.deepseek.com/chat/completions",
        chain: &["deepseek-chat", "deepseek-reasoner"],
        chain_env: "CREW_DEEPSEEK_MODEL",
        base_url_env: "CREW_DEEPSEEK_BASE_URL",
        vendor: crew_hive::catalog::Vendor::DeepSeek,
    },
];

/// The `DIRECT` row named by `name`, if any.
pub fn direct_by_name(name: &str) -> Option<&'static DirectProvider> {
    DIRECT.iter().find(|d| d.name.eq_ignore_ascii_case(name))
}

/// The provider backing the project's API-backed agents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProviderKind {
    Mock,
    DashScope,
    OpenRouter,
    Anthropic,
    /// One of the [`DIRECT`] rows. Carried by reference because the table is
    /// static, so this stays `Copy` and comparisons stay cheap.
    Direct(&'static DirectProvider),
}

impl ProviderKind {
    /// How this provider is named to a user and in `CREW_PROVIDER`. Not the
    /// `Debug` spelling: that nests the table row inside the variant and
    /// `/doctor` printed `direct(direct(openai))` for a while because of it.
    pub fn name(self) -> &'static str {
        match self {
            ProviderKind::Mock => "mock",
            ProviderKind::DashScope => "dashscope",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Direct(d) => d.name,
        }
    }
}

impl Clone for DirectProvider {
    fn clone(&self) -> Self {
        unreachable!("DirectProvider lives in a static table and is never cloned")
    }
}

impl PartialEq for DirectProvider {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for DirectProvider {}
impl std::fmt::Debug for DirectProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Direct({})", self.name)
    }
}

/// Resolve which provider backs the project's API-backed agents. The mock
/// (tests) always wins; then an explicit `CREW_PROVIDER`
/// (dashscope|openrouter|anthropic); then auto-discovery in preference order
/// — DashScope (paid Qwen) before OpenRouter (free chains) before Anthropic.
pub fn pick_provider(force: Option<&str>, has_key: impl Fn(&str) -> bool) -> Option<ProviderKind> {
    if has_key("CREW_BROKER_MOCK_REPLY") {
        return Some(ProviderKind::Mock);
    }
    match force.map(str::to_ascii_lowercase).as_deref() {
        Some("dashscope") => return Some(ProviderKind::DashScope),
        Some("openrouter") => return Some(ProviderKind::OpenRouter),
        Some("anthropic") => return Some(ProviderKind::Anthropic),
        Some(other) => {
            if let Some(d) = direct_by_name(other) {
                return Some(ProviderKind::Direct(d));
            }
        }
        _ => {}
    }
    if has_key("DASHSCOPE_API_KEY") {
        Some(ProviderKind::DashScope)
    } else if has_key("OPENROUTER_API_KEY") {
        Some(ProviderKind::OpenRouter)
    } else if has_key("ANTHROPIC_API_KEY") {
        Some(ProviderKind::Anthropic)
    } else {
        // New providers probe LAST, so adding a row can never change which
        // provider an existing install resolves to.
        DIRECT
            .iter()
            .find(|d| has_key(d.var))
            .map(ProviderKind::Direct)
    }
}

/// Which provider is pinned: `CREW_PROVIDER` if it names one, else the pin the
/// credential store recorded when a key was saved. Env wins, so an explicit
/// `CREW_PROVIDER=… crew` is never overridden by something crew stored itself.
///
/// Takes an already-loaded `store` so one request reads the file once — for
/// the pin AND the key it goes with (see [`key_for`]) — rather than once per
/// lookup.
fn forced_provider(store: &crate::credentials::Store) -> Option<String> {
    resolve_forced(std::env::var("CREW_PROVIDER").ok(), store.provider.clone())
}

/// The pure half of [`forced_provider`], so the precedence is testable without
/// the process environment or the real config directory.
fn resolve_forced(env: Option<String>, stored: Option<String>) -> Option<String> {
    env.filter(|v| !v.is_empty())
        .or_else(|| stored.filter(|v| !v.is_empty()))
}

/// The value backing `var` for THIS request: the process environment first,
/// then the credential store.
///
/// Read per request, exactly like the pin in [`forced_provider`] — and for the
/// same reason. `shellenv::hydrate()` imports the store into the environment
/// ONCE per broker process, at startup; the broker child is spawned when the
/// chat pane opens, and the key prompt only exists *inside* a chat pane, so a
/// key typed into crew is always written after the only import that would have
/// picked it up. Reading only the environment here meant a saved key never
/// reached the running broker at all — and, because saving one also writes the
/// provider pin (which IS re-read per request), it actively broke a working
/// session: `pick_provider` would return the pinned provider, this lookup
/// would find nothing, and `roster_with` would fall back to plugins only,
/// silently dropping the user's specialists until the pane was reopened.
///
/// Never logs the value.
fn key_for(store: &crate::credentials::Store, var: &str) -> Option<String> {
    resolve_key_with(
        std::env::var(var).ok(),
        store.keys.get(var).cloned(),
        super::shellenv::crew_injected(var),
    )
}

/// The pure half of [`key_for`]. A variable exported non-empty into this
/// process WINS over the stored value — the most deliberate signal a user can
/// send, and the same precedence `shellenv::credential_imports` already
/// applies when it imports the store at startup.
///
/// `env_is_crew_injected` says whether the environment's value is crew's OWN
/// injection rather than the user's.
///
/// `shellenv::hydrate` copies stored keys into this process's environment so
/// child processes inherit them, but it runs once per broker process while the
/// store is re-read every request. Without this distinction, rotating a key in
/// a later session would update the store and change nothing: crew's startup
/// injection would outrank it on every request, and the user would get 401s
/// they could not fix without quitting crew. So for a variable crew injected
/// itself, the store wins — it is where that value came from in the first
/// place. The environment still wins for anything the USER exported.
fn resolve_key_with(
    env: Option<String>,
    stored: Option<String>,
    env_is_crew_injected: bool,
) -> Option<String> {
    let env = env.filter(|v| !v.is_empty());
    let stored = stored.filter(|v| !v.is_empty());
    if env_is_crew_injected {
        // Fall back to the environment when the store has since been emptied,
        // so a half-removed credential degrades to the old behaviour rather
        // than to no provider at all.
        return stored.or(env);
    }
    env.or(stored)
}

/// The provider this request resolves to, reading the pin and the keys the
/// same way: process environment first, credential store second. The single
/// answer to "which provider is active right now" — [`roster_with`],
/// `stdio::provider_resolves` and `doctor::gather` all come through here, so
/// none of them can disagree with the roster that actually gets built.
pub(crate) fn resolved_provider() -> Option<ProviderKind> {
    picked(&crate::credentials::load())
}

/// [`resolved_provider`] over an already-loaded store.
fn picked(store: &crate::credentials::Store) -> Option<ProviderKind> {
    pick_provider(forced_provider(store).as_deref(), |k| {
        key_for(store, k).is_some()
    })
}

/// The full adapter roster, in precedence order: stored specialists (see
/// [`super::specialists`]) composed over the picked provider — or, with no
/// provider, none at all — then every installed manifest plugin agent (see
/// [`super::plugins`]), then every installed built-in CLI agent (see
/// [`super::agents::append_installed`]).
///
/// The last two are appended whether or not a provider resolved, because
/// neither needs an API key: a manifest agent and a `claude`/`codex`/
/// `opencode` install both shell out to a CLI that carries its own sign-in.
/// A user with zero keys is therefore not a user with zero agents — which is
/// the whole point, and was not true until v0.6.39 despite the built-in
/// adapters having existed all along with no caller.
///
/// Manifests come before built-ins so an explicit local declaration wins the
/// name. The mock roster stays free of both so end-to-end tests are
/// deterministic on any machine.
pub(crate) fn roster_with(
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Adapter>> {
    let store = crate::credentials::load();
    let mut agents = match provider_and_model_with(&store, crew_hive::ModelTier::Standard) {
        Some((provider, model)) => specialist_agents(provider, &model, overrides),
        None => Vec::new(),
    };
    // The mock roster stays plugin- and CLI-free so end-to-end tests are
    // deterministic on any machine.
    if !matches!(picked(&store), Some(ProviderKind::Mock)) {
        super::plugins::append(&mut agents);
        super::agents::append_installed(&mut agents);
    }
    agents
}

/// [`provider_and_model`] with an explicit tier. Only Anthropic maps a tier to
/// a model id — DashScope and OpenRouter default to their chain head
/// (`chain[0]`), so `tier` is ignored there. Serves both the Far pane's
/// one-shot `!` command suggestion (via `provider_and_model`, pinned to
/// `Cheap`) and the specialist roster (`roster_with`, pinned to `Standard`).
pub(crate) fn provider_and_model_for(
    tier: crew_hive::ModelTier,
) -> Option<(Arc<dyn crew_hive::Provider>, String)> {
    provider_and_model_with(&crate::credentials::load(), tier)
}

/// [`provider_and_model_for`] over an already-loaded credential store: one
/// read of the store answers both "which provider" and "with which key", so a
/// request can never resolve a provider from the store and then fail to find
/// the key that goes with it.
fn provider_and_model_with(
    store: &crate::credentials::Store,
    tier: crew_hive::ModelTier,
) -> Option<(Arc<dyn crew_hive::Provider>, String)> {
    match picked(store)? {
        ProviderKind::Mock => {
            let reply = std::env::var("CREW_BROKER_MOCK_REPLY").unwrap_or_default();
            let provider = crew_hive::MockProvider { reply };
            Some((
                Arc::new(provider) as Arc<dyn crew_hive::Provider>,
                "mock".to_string(),
            ))
        }
        ProviderKind::DashScope => {
            let key = key_for(store, "DASHSCOPE_API_KEY")?;
            let chain = parse_model_chain(
                std::env::var("CREW_DASHSCOPE_MODEL").ok(),
                DEFAULT_DASHSCOPE_CHAIN
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            );
            let url = std::env::var("CREW_DASHSCOPE_BASE_URL")
                .unwrap_or_else(|_| DASHSCOPE_ENDPOINT.to_string());
            let model = chain.first().cloned()?;
            let provider = crew_hive::OpenRouterProvider::new(key)
                .with_endpoint(url)
                .with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::OpenRouter => {
            // Not `from_env()`: that reads the environment only, and the key
            // may live in the store (see [`key_for`]).
            let provider =
                crew_hive::OpenRouterProvider::new(key_for(store, "OPENROUTER_API_KEY")?);
            let chain = parse_model_chain(
                std::env::var("CREW_OPENROUTER_MODEL").ok(),
                default_openrouter_chain(),
            );
            let model = chain.first().cloned()?;
            let provider = provider.with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::Direct(d) => {
            let key = key_for(store, d.var)?;
            let chain = parse_model_chain(
                std::env::var(d.chain_env).ok(),
                d.chain.iter().map(|s| s.to_string()).collect(),
            );
            let url = std::env::var(d.base_url_env).unwrap_or_else(|_| d.endpoint.to_string());
            let model = chain.first().cloned()?;
            let provider = crew_hive::OpenRouterProvider::new(key)
                .with_endpoint(url)
                .with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::Anthropic => {
            // Not `from_env()` — same reason as OpenRouter above.
            let provider = crew_hive::AnthropicProvider::new(key_for(store, "ANTHROPIC_API_KEY")?);
            Some((
                Arc::new(provider) as Arc<dyn crew_hive::Provider>,
                tier.model_id().to_string(),
            ))
        }
    }
}

/// The default provider + a cheap model, for one-shot low-token asks that
/// need a custom system prompt and a small `max_tokens` — neither of which the
/// `Adapter` trait exposes (`ApiAdapter::call` always sends the role's fixed
/// system prompt and a 2048-token ceiling). Used by the Far pane's `!` command
/// suggestion ([`super::ask::suggest_far_command`]): a one-line shell hint
/// needs no deep reasoning, hence `ModelTier::Cheap`.
pub(crate) fn provider_and_model() -> Option<(Arc<dyn crew_hive::Provider>, String)> {
    provider_and_model_for(crew_hive::ModelTier::Cheap)
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
