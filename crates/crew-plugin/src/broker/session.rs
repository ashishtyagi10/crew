//! Mutable per-connection broker state: settings the user changes with slash
//! constructs (per-agent model overrides, …) that must survive across sends
//! for as long as the `/crew` pane is open.
use std::collections::HashMap;

use super::Registry;

#[derive(Default)]
pub(crate) struct Session {
    /// Per-agent model overrides (`agent name → model id`), set by `/model`.
    /// Agents without an entry run their provider default, so different agents
    /// can run different models side by side.
    pub overrides: HashMap<String, String>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// The agent registry with this session's model overrides applied.
    pub fn registry(&self) -> Registry {
        Registry::discover_with(&self.overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_no_overrides() {
        assert!(Session::new().overrides.is_empty());
    }
}
