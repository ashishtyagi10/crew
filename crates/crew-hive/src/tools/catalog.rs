//! Provider-facing tool definitions, and the map back to crew's own spelling.
//!
//! Split out of `tools/mod.rs` so the trait file stays inside the line cap as the
//! trait grew its retrieval seams; the type is exactly as it was there.
use super::ToolSpec;

/// Provider-facing tool definitions plus the map back to `(server, tool)`.
///
/// The map is why this is a type and not a function: decoding by splitting the
/// wire name would be a guess, and a guess is wrong the moment a name is
/// sanitised, truncated or de-duplicated. Whatever transformation happened on
/// the way out, [`Self::resolve`] undoes exactly.
#[derive(Debug, Default)]
pub struct ToolCatalog {
    defs: Vec<crate::provider::ToolDef>,
    by_wire: std::collections::HashMap<String, (String, String)>,
}

/// Longest function name providers accept.
const MAX_WIRE_NAME: usize = 64;

/// One side of a wire name: every character outside `[A-Za-z0-9_-]` becomes
/// `_`. Note that `:` and `.` — the two most likely characters in a real MCP
/// server name — both land here.
fn sanitize(part: &str) -> String {
    part.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl ToolCatalog {
    /// Encode `specs` for the wire, keeping every name unique and legal.
    ///
    /// Uniqueness is enforced rather than assumed: two servers whose names
    /// differ only in a character that sanitises to `_` would otherwise
    /// collide, and a collision means a model's call resolves to the WRONG
    /// TOOL — silently, on someone else's server. A clashing name gets a
    /// numeric suffix, and the map records where it really goes.
    pub fn build(specs: &[ToolSpec]) -> Self {
        let mut defs = Vec::with_capacity(specs.len());
        let mut by_wire: std::collections::HashMap<String, (String, String)> =
            std::collections::HashMap::new();
        for spec in specs {
            let base = format!("{}__{}", sanitize(&spec.server), sanitize(&spec.tool));
            let base: String = base.chars().take(MAX_WIRE_NAME).collect();
            let mut name = base.clone();
            let mut n = 2;
            while by_wire.contains_key(&name) {
                let suffix = format!("_{n}");
                let keep = MAX_WIRE_NAME - suffix.len();
                name = format!("{}{suffix}", base.chars().take(keep).collect::<String>());
                n += 1;
            }
            by_wire.insert(name.clone(), (spec.server.clone(), spec.tool.clone()));
            defs.push(crate::provider::ToolDef {
                name,
                description: spec.description.clone(),
                input_schema: spec.input_schema.clone(),
            });
        }
        Self { defs, by_wire }
    }

    pub fn defs(&self) -> &[crate::provider::ToolDef] {
        &self.defs
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// `(server, tool)` for a name the model called, or `None` if it invented
    /// one — which models do, and which must read as a tool error the agent
    /// can recover from rather than a panic or a call to something else.
    pub fn resolve(&self, wire: &str) -> Option<(&str, &str)> {
        self.by_wire
            .get(wire)
            .map(|(s, t)| (s.as_str(), t.as_str()))
    }

    /// The names a model may call, for an error message that helps.
    pub fn names(&self) -> Vec<&str> {
        self.defs.iter().map(|d| d.name.as_str()).collect()
    }
}
