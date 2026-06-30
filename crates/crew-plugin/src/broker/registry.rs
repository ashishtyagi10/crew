//! The agent registry: maps a name to its adapter. [`Registry::discover`]
//! probes every known agent at runtime and keeps only the installed ones, so
//! the broker never routes to a CLI that isn't there.
use std::sync::Arc;

use super::adapter::Adapter;
use super::apiadapter::inbuilt_agents;

/// Default OpenRouter model for the inbuilt agents — a free slug, since quality
/// is not the current goal. OpenRouter rotates its free models, so override with
/// `CREW_OPENROUTER_MODEL=<slug>` if this one is retired.
const DEFAULT_OPENROUTER_MODEL: &str = "meta-llama/llama-3.3-70b-instruct:free";

pub struct Registry {
    agents: Vec<Box<dyn Adapter>>,
}

impl Registry {
    /// Wrap an explicit set of adapters (used by tests with fake agents).
    pub fn new(agents: Vec<Box<dyn Adapter>>) -> Self {
        Self { agents }
    }

    /// Build the inbuilt agent roster (planner/coder/reviewer). Prefers OpenRouter
    /// (`OPENROUTER_API_KEY`) — every role runs on [`DEFAULT_OPENROUTER_MODEL`],
    /// overridable with `CREW_OPENROUTER_MODEL` — then falls back to Anthropic
    /// (`ANTHROPIC_API_KEY`, per-tier native models). Empty when neither key is
    /// set, so the broker explains how to enable it rather than routing to nothing.
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
            let model = std::env::var("CREW_OPENROUTER_MODEL")
                .unwrap_or_else(|_| DEFAULT_OPENROUTER_MODEL.to_string());
            // Quality is not the goal here (a free model by default); every role
            // shares one OpenRouter slug, with the role's system prompt steering it.
            return Self::new(inbuilt_agents(Arc::new(provider), move |_| model.clone()));
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
mod tests {
    use super::*;
    use std::time::Duration;

    struct Stub(&'static str);
    impl Adapter for Stub {
        fn name(&self) -> &str {
            self.0
        }
        fn probe(&self) -> bool {
            true
        }
        fn call(&self, _body: &str, _t: Duration) -> Result<String, String> {
            Ok(String::new())
        }
    }

    fn reg() -> Registry {
        Registry::new(vec![Box::new(Stub("claude")), Box::new(Stub("codex"))])
    }

    #[test]
    fn get_is_case_insensitive() {
        assert!(reg().get("Claude").is_some());
        assert!(reg().get("nope").is_none());
    }

    #[test]
    fn peers_excludes_self() {
        assert_eq!(reg().peers_of("claude"), vec!["codex".to_string()]);
    }

    #[test]
    fn roster_excluding_adds_role_hints() {
        let roster = reg().roster_excluding("claude");
        assert_eq!(roster.len(), 1);
        assert!(roster[0].starts_with("codex ("), "{}", roster[0]);
    }

    #[test]
    fn names_and_len() {
        let r = reg();
        assert_eq!(r.len(), 2);
        assert_eq!(r.names(), vec!["claude", "codex"]);
        assert!(!r.is_empty());
    }
}
