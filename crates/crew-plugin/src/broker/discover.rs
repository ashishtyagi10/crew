//! Roster discovery: which provider backs the inbuilt agents, and the final
//! adapter list (inbuilt API agents + manifest plugin agents). Split from
//! `registry` to keep both under the line cap.
use std::sync::Arc;

use super::adapter::Adapter;
use super::apiadapter::inbuilt_agents;

/// Default OpenRouter fallback chain for the inbuilt agents — free slugs across
/// *different* upstream providers, so a provider-specific throttle on one model
/// rolls to the next instead of failing the relay. Quality isn't the goal here.
/// OpenRouter rotates its free models; override the whole chain with a
/// comma-separated `CREW_OPENROUTER_MODEL=slug1,slug2,…` (a retired slug is
/// skipped automatically when it errors).
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

/// The provider backing the inbuilt agents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ProviderKind {
    Mock,
    DashScope,
    OpenRouter,
    Anthropic,
}

/// Resolve which provider backs the inbuilt agents. The mock (tests) always
/// wins; then an explicit `CREW_PROVIDER` (dashscope|openrouter|anthropic);
/// then auto-discovery in preference order — DashScope (paid Qwen) before
/// OpenRouter (free chains) before Anthropic.
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

/// The full adapter roster: the picked provider's inbuilt agents, then every
/// installed manifest plugin agent (see [`super::plugins`]). The mock roster
/// stays plugin-free so end-to-end tests are deterministic on any machine.
pub(crate) fn roster_with(
    overrides: &std::collections::HashMap<String, String>,
) -> Vec<Box<dyn Adapter>> {
    let force = std::env::var("CREW_PROVIDER").ok();
    let has = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty());
    let mut agents = match pick_provider(force.as_deref(), has) {
        Some(ProviderKind::Mock) => {
            let reply = std::env::var("CREW_BROKER_MOCK_REPLY").unwrap_or_default();
            let provider = Arc::new(crew_hive::MockProvider { reply });
            return inbuilt_agents(provider, |t| t.model_id().to_string(), overrides);
        }
        // Alibaba Cloud DashScope: the same OpenAI-compatible wire shape on
        // a different endpoint, running the Qwen commercial models.
        Some(ProviderKind::DashScope) => match std::env::var("DASHSCOPE_API_KEY") {
            Err(_) => Vec::new(), // forced without a key
            Ok(key) => {
                let chain = parse_model_chain(
                    std::env::var("CREW_DASHSCOPE_MODEL").ok(),
                    DEFAULT_DASHSCOPE_CHAIN,
                );
                let url = std::env::var("CREW_DASHSCOPE_BASE_URL")
                    .unwrap_or_else(|_| DASHSCOPE_ENDPOINT.to_string());
                let primary = chain[0].clone();
                let provider = crew_hive::OpenRouterProvider::new(key)
                    .with_endpoint(url)
                    .with_fallbacks(chain);
                inbuilt_agents(Arc::new(provider), move |_| primary.clone(), overrides)
            }
        },
        Some(ProviderKind::OpenRouter) => match crew_hive::OpenRouterProvider::from_env() {
            Err(_) => Vec::new(), // forced without a key
            Ok(provider) => {
                let chain = parse_model_chain(
                    std::env::var("CREW_OPENROUTER_MODEL").ok(),
                    DEFAULT_OPENROUTER_CHAIN,
                );
                // Every role starts on the chain's first slug (the role's system
                // prompt steers it); the provider rolls to later slugs when one
                // is limited.
                let primary = chain[0].clone();
                let provider = provider.with_fallbacks(chain);
                inbuilt_agents(Arc::new(provider), move |_| primary.clone(), overrides)
            }
        },
        Some(ProviderKind::Anthropic) => match crew_hive::AnthropicProvider::from_env() {
            Err(_) => Vec::new(), // forced without a key
            Ok(provider) => {
                inbuilt_agents(Arc::new(provider), |t| t.model_id().to_string(), overrides)
            }
        },
        None => Vec::new(),
    };
    super::plugins::append(&mut agents);
    agents
}

/// Resolve the default provider + a single reasonable model id, without
/// building the full inbuilt-agent roster (`roster_with`) or its `Adapter`
/// wrapping. For one-shot low-token asks that need a custom system prompt
/// and a small `max_tokens` — neither of which the `Adapter` trait exposes
/// (`ApiAdapter::call` always sends the role's fixed system prompt and a
/// 2048-token ceiling). Used by the Far pane's `!` command suggestion
/// ([`super::ask::suggest_far_command`]). Mirrors `roster_with`'s branches
/// exactly (mock / DashScope / OpenRouter / Anthropic), picking
/// `ModelTier::Cheap` for Anthropic — a one-line shell suggestion needs no
/// deep reasoning — while DashScope/OpenRouter already default to their
/// cheapest usable chain head (`chain[0]`), so no tier mapping applies there.
pub(crate) fn provider_and_model() -> Option<(Arc<dyn crew_hive::Provider>, String)> {
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
                crew_hive::ModelTier::Cheap.model_id().to_string(),
            ))
        }
    }
}

#[cfg(test)]
#[path = "discover_tests.rs"]
mod tests;
