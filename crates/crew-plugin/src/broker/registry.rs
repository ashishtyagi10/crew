//! The agent registry: maps a name to its adapter. [`Registry::discover`]
//! resolves the provider-backed inbuilt roster plus any manifest plugin
//! agents (see [`super::discover`]), keeping only usable ones, so the broker
//! never routes to an agent that isn't there.
use super::adapter::Adapter;

pub struct Registry {
    agents: Vec<Box<dyn Adapter>>,
}

impl Registry {
    /// Wrap an explicit set of adapters (used by tests with fake agents).
    pub fn new(agents: Vec<Box<dyn Adapter>>) -> Self {
        Self { agents }
    }

    /// Build the roster: the inbuilt API agents (planner/coder/reviewer) on
    /// the discovered provider — `CREW_PROVIDER` forces one; otherwise keys
    /// are probed DashScope → OpenRouter → Anthropic — plus every installed
    /// manifest plugin agent from `~/.config/crew/agents/` and
    /// `./.crew/agents/`. With no key and no plugin agents the roster is
    /// empty, so the broker explains how to enable it rather than routing to
    /// nothing. `CREW_BROKER_MOCK_REPLY` overrides everything with a
    /// fixed-reply mock (no network, no plugins) for deterministic tests.
    pub fn discover() -> Self {
        Self::discover_with(&std::collections::HashMap::new())
    }

    /// [`Registry::discover`] with per-agent model overrides (the `/model`
    /// construct) applied on top of the provider's defaults, so different
    /// agents can run different models side by side.
    pub fn discover_with(overrides: &std::collections::HashMap<String, String>) -> Self {
        Self::new(super::discover::roster_with(overrides))
    }

    /// Registered agent names, in registration order.
    /// This registry with only `name` in it — a turn that is one agent's
    /// alone, with nobody to hand off to. Unchanged when `name` is absent.
    pub fn only(mut self, name: &str) -> Self {
        if self
            .agents
            .iter()
            .any(|a| a.name().eq_ignore_ascii_case(name))
        {
            self.agents.retain(|a| a.name().eq_ignore_ascii_case(name));
        }
        self
    }

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
                role: a.role().to_string(),
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
        self.agents
            .iter()
            .filter(|a| !a.name().eq_ignore_ascii_case(name))
            .map(|a| match a.role() {
                "" => a.name().to_string(),
                role => format!("{} ({role})", a.name()),
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
