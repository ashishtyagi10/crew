//! Roster discovery: which provider backs the project's stored specialists
//! (see [`super::specialists`]), and the final adapter list (API-backed
//! specialist agents + manifest plugin agents). Split from `registry` to keep
//! both under the line cap.
use std::sync::Arc;

use super::adapter::Adapter;
use super::apiadapter::specialist_agents;

/// Default OpenRouter fallback chain for the project's API-backed agents —
/// free slugs across *different* upstream providers, so a provider-specific
/// throttle on one model rolls to the next instead of failing the relay.
/// Quality isn't the goal here. OpenRouter rotates its free models; override
/// the whole chain with a comma-separated `CREW_OPENROUTER_MODEL=slug1,slug2,…`
/// (a retired slug is skipped automatically when it errors).
pub(crate) const DEFAULT_OPENROUTER_CHAIN: &[&str] = &[
    "meta-llama/llama-3.3-70b-instruct:free",
    "deepseek/deepseek-chat-v3.1:free",
    "qwen/qwen3-235b-a22b:free",
    "meta-llama/llama-4-scout:free",
];

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
pub(crate) fn parse_model_chain(env_val: Option<String>, default: &[&str]) -> Vec<String> {
    let parsed: Vec<String> = env_val
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parsed.is_empty() {
        default.iter().map(|s| s.to_string()).collect()
    } else {
        parsed
    }
}

/// The provider backing the project's API-backed agents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ProviderKind {
    Mock,
    DashScope,
    OpenRouter,
    Anthropic,
}

/// Resolve which provider backs the project's API-backed agents. The mock
/// (tests) always wins; then an explicit `CREW_PROVIDER`
/// (dashscope|openrouter|anthropic); then auto-discovery in preference order
/// — DashScope (paid Qwen) before OpenRouter (free chains) before Anthropic.
pub(crate) fn pick_provider(
    force: Option<&str>,
    has_key: impl Fn(&str) -> bool,
) -> Option<ProviderKind> {
    if has_key("CREW_BROKER_MOCK_REPLY") {
        return Some(ProviderKind::Mock);
    }
    match force.map(str::to_ascii_lowercase).as_deref() {
        Some("dashscope") => return Some(ProviderKind::DashScope),
        Some("openrouter") => return Some(ProviderKind::OpenRouter),
        Some("anthropic") => return Some(ProviderKind::Anthropic),
        _ => {}
    }
    if has_key("DASHSCOPE_API_KEY") {
        Some(ProviderKind::DashScope)
    } else if has_key("OPENROUTER_API_KEY") {
        Some(ProviderKind::OpenRouter)
    } else if has_key("ANTHROPIC_API_KEY") {
        Some(ProviderKind::Anthropic)
    } else {
        None
    }
}

/// The full adapter roster: stored specialists (see [`super::specialists`])
/// composed over the picked provider — or, with no provider, none at all —
/// then every installed manifest plugin agent (see [`super::plugins`])
/// appended in *either* case. Plugin agents shell out to an installed CLI and
/// need no API key, so a user with zero keys but a `.crew/agents/` manifest
/// still gets a working, plugin-only roster instead of an empty one. The mock
/// roster stays plugin-free so end-to-end tests are deterministic on any
/// machine.
pub(crate) fn roster_with(
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Adapter>> {
    let mut agents = match provider_and_model_for(crew_hive::ModelTier::Standard) {
        Some((provider, model)) => specialist_agents(provider, &model, overrides),
        None => Vec::new(),
    };
    // The mock roster stays plugin-free so end-to-end tests are deterministic
    // on any machine.
    if !matches!(
        pick_provider(std::env::var("CREW_PROVIDER").ok().as_deref(), |k| {
            std::env::var(k).is_ok_and(|v| !v.is_empty())
        }),
        Some(ProviderKind::Mock)
    ) {
        super::plugins::append(&mut agents);
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
    let force = std::env::var("CREW_PROVIDER").ok();
    let has = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    match pick_provider(force.as_deref(), has)? {
        ProviderKind::Mock => {
            let reply = std::env::var("CREW_BROKER_MOCK_REPLY").unwrap_or_default();
            let provider = crew_hive::MockProvider { reply };
            Some((
                Arc::new(provider) as Arc<dyn crew_hive::Provider>,
                "mock".to_string(),
            ))
        }
        ProviderKind::DashScope => {
            let key = std::env::var("DASHSCOPE_API_KEY").ok()?;
            let chain = parse_model_chain(
                std::env::var("CREW_DASHSCOPE_MODEL").ok(),
                DEFAULT_DASHSCOPE_CHAIN,
            );
            let url = std::env::var("CREW_DASHSCOPE_BASE_URL")
                .unwrap_or_else(|_| DASHSCOPE_ENDPOINT.to_string());
            let model = chain[0].clone();
            let provider = crew_hive::OpenRouterProvider::new(key)
                .with_endpoint(url)
                .with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::OpenRouter => {
            let provider = crew_hive::OpenRouterProvider::from_env().ok()?;
            let chain = parse_model_chain(
                std::env::var("CREW_OPENROUTER_MODEL").ok(),
                DEFAULT_OPENROUTER_CHAIN,
            );
            let model = chain[0].clone();
            let provider = provider.with_fallbacks(chain);
            Some((Arc::new(provider) as Arc<dyn crew_hive::Provider>, model))
        }
        ProviderKind::Anthropic => {
            let provider = crew_hive::AnthropicProvider::from_env().ok()?;
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
