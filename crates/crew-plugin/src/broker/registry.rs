//! The agent registry: maps a name to its adapter. [`Registry::discover`]
//! probes every known agent at runtime and keeps only the installed ones, so
//! the broker never routes to a CLI that isn't there.
use std::sync::Arc;

use super::adapter::Adapter;
use super::apiadapter::inbuilt_agents;

/// Default OpenRouter fallback chain for the inbuilt agents — free slugs across
/// *different* upstream providers, so a provider-specific throttle on one model
/// rolls to the next instead of failing the relay. Quality isn't the goal here.
/// OpenRouter rotates its free models; override the whole chain with a
/// comma-separated `CREW_OPENROUTER_MODEL=slug1,slug2,…` (a retired slug is
/// skipped automatically when it errors).
const DEFAULT_OPENROUTER_CHAIN: &[&str] = &[
    "meta-llama/llama-3.3-70b-instruct:free",
    "deepseek/deepseek-chat-v3.1:free",
    "qwen/qwen3-235b-a22b:free",
    "meta-llama/llama-4-scout:free",
];

/// Parse `CREW_OPENROUTER_MODEL` (a comma-separated model chain) into an ordered
/// list, falling back to `default` when unset or empty.
fn parse_model_chain(env_val: Option<String>, default: &[&str]) -> Vec<String> {
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

pub struct Registry {
    agents: Vec<Box<dyn Adapter>>,
}

impl Registry {
    /// Wrap an explicit set of adapters (used by tests with fake agents).
    pub fn new(agents: Vec<Box<dyn Adapter>>) -> Self {
        Self { agents }
    }

    /// Build the inbuilt agent roster (planner/coder/reviewer). Prefers OpenRouter
    /// (`OPENROUTER_API_KEY`) — every role runs on the [`DEFAULT_OPENROUTER_CHAIN`]
    /// (or a `CREW_OPENROUTER_MODEL` override), auto-falling-back model to model on
    /// rate-limits — then Anthropic (`ANTHROPIC_API_KEY`, per-tier native models).
    /// Empty when neither key is set, so the broker explains how to enable it
    /// rather than routing to nothing.
    ///
    /// `CREW_BROKER_MOCK_REPLY` overrides the provider with a fixed-reply mock
    /// (no network), so the relay can be driven deterministically offline and in
    /// end-to-end tests of the broker binary.
    pub fn discover() -> Self {
        if let Ok(reply) = std::env::var("CREW_BROKER_MOCK_REPLY") {
            let provider = Arc::new(crew_hive::MockProvider { reply });
            return Self::new(inbuilt_agents(provider, |t| t.model_id().to_string()));
        }
        if let Ok(provider) = crew_hive::OpenRouterProvider::from_env() {
            let chain = parse_model_chain(
                std::env::var("CREW_OPENROUTER_MODEL").ok(),
                DEFAULT_OPENROUTER_CHAIN,
            );
            // Every role starts on the chain's first slug (the role's system prompt
            // steers it); the provider rolls to later slugs when one is limited.
            let primary = chain[0].clone();
            let provider = provider.with_fallbacks(chain);
            return Self::new(inbuilt_agents(Arc::new(provider), move |_| primary.clone()));
        }
        if let Ok(provider) = crew_hive::AnthropicProvider::from_env() {
            return Self::new(inbuilt_agents(Arc::new(provider), |t| {
                t.model_id().to_string()
            }));
        }
        Self::new(Vec::new())
    }

    /// Registered agent names, in registration order.
    pub fn names(&self) -> Vec<String> {
        self.agents.iter().map(|a| a.name().to_string()).collect()
    }

    /// Roster entries (name, role hint, model) for every registered agent, in
    /// registration order — the payload of the host-facing `Roster` event.
    pub fn infos(&self) -> Vec<crate::AgentInfo> {
        self.agents
            .iter()
            .map(|a| crate::AgentInfo {
                name: a.name().to_string(),
                role: super::agents::role_for(a.name()).to_string(),
                model: a.model().to_string(),
            })
            .collect()
    }

    /// Look up an agent by name, case-insensitively.
    pub fn get(&self, name: &str) -> Option<&dyn Adapter> {
        self.agents
            .iter()
            .find(|a| a.name().eq_ignore_ascii_case(name))
            .map(|b| b.as_ref())
    }

    /// Names of every registered agent except `name` (its potential peers).
    pub fn peers_of(&self, name: &str) -> Vec<String> {
        self.names()
            .into_iter()
            .filter(|n| !n.eq_ignore_ascii_case(name))
            .collect()
    }

    /// Peer descriptions (name + capability hint) for everyone except `name` —
    /// the prompt's peer list, so an agent hands off to the right one.
    pub fn roster_excluding(&self, name: &str) -> Vec<String> {
        self.names()
            .into_iter()
            .filter(|n| !n.eq_ignore_ascii_case(name))
            .map(|n| match super::agents::role_for(&n) {
                "" => n,
                role => format!("{n} ({role})"),
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
