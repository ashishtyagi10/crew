//! Integrations: an HTTP API becomes a set of tools from ONE FILE, with no Rust.
//!
//! Goal: `docs/superpowers/goals/2026-09-01-close-the-open-goals.md`, Pillar 3 — *adding the
//! fortieth integration must cost a manifest and a `/reload`*. crew already proves the pattern
//! three times over: plugin agents load from `~/.config/crew/agents/`, skills from
//! `.crew/skills`, MCP servers from `mcp.json`. Tools were the one extension surface that never
//! got it, so reaching a new API meant editing `systools.rs` and cutting a release.
//!
//! A manifest is a server of tools:
//!
//! ```json
//! {
//!   "name": "weather",
//!   "base_url": "https://api.open-meteo.com/v1",
//!   "auth": {"kind": "bearer", "env": "WEATHER_TOKEN"},
//!   "tools": [{
//!     "name": "forecast",
//!     "description": "the forecast for a place",
//!     "method": "GET",
//!     "path": "/forecast",
//!     "query": {"latitude": "{lat}", "longitude": "{lon}"},
//!     "tier": "read",
//!     "input_schema": {"type": "object", "properties": {"lat": {"type": "string"}}}
//!   }]
//! }
//! ```
//!
//! Two rules the format enforces rather than suggests:
//!
//! * **A secret is never in the file.** `auth` names an ENVIRONMENT VARIABLE; there is no field
//!   that takes a token. Manifests get copied between machines, pasted into issues and committed
//!   to `.crew/` in a repo, and a format with a `"token"` field is a format that leaks one.
//! * **A tool is irreversible until its manifest says otherwise.** `tier` defaults to the
//!   strictest class, so an integration nobody has thought carefully about asks before acting
//!   rather than after — the same default an unknown MCP server gets.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::mcp::McpTool;

pub(crate) mod args;
pub(crate) mod request;
pub(crate) mod run;

/// How the API is authenticated. Every variant names an env var; none carries a secret.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum Auth {
    /// `Authorization: Bearer $env`.
    Bearer { env: String },
    /// A header of your own: `X-Api-Key: $env`.
    Header { name: String, env: String },
    /// A query parameter: `?key=$env`.
    Query { name: String, env: String },
    /// A public API.
    #[default]
    None,
}

/// One tool the integration offers.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct IntTool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// `GET`, `POST`, `PUT`, `PATCH`, `DELETE`. Defaults to `GET`.
    #[serde(default = "get")]
    pub method: String,
    /// Appended to `base_url`. `{arg}` placeholders are filled from the call's arguments.
    #[serde(default)]
    pub path: String,
    /// Query parameters, values with the same `{arg}` substitution.
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    /// A JSON body template, with `{arg}` substitution inside string values. Absent for a GET.
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    /// JSON Schema for the arguments, handed to the model as-is.
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    /// `read`, `reversible`, `irreversible`. Absent means irreversible.
    #[serde(default)]
    pub tier: Option<String>,
}

fn get() -> String {
    "GET".into()
}

/// One manifest: a named server with a base URL and its tools.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub(crate) struct Integration {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub base_url: String,
    #[serde(default)]
    pub auth: Auth,
    /// Extra headers sent on every call — a `User-Agent`, an API version. Values are literal;
    /// a secret belongs in `auth`, which reads the environment.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub tools: Vec<IntTool>,
}

impl Integration {
    /// The tier of one of its tools, as [`super::tier`] classifies things.
    pub(crate) fn tier_of(&self, tool: &str) -> super::tier::Tier {
        use super::tier::Tier;
        match self
            .tools
            .iter()
            .find(|t| t.name == tool)
            .and_then(|t| t.tier.as_deref())
        {
            Some("read") => Tier::Read,
            Some("reversible") => Tier::Reversible,
            // Including an unrecognised word: a typo in a tier must not read as permission.
            _ => Tier::Irreversible,
        }
    }
}

/// Every integration's tools, in the shape the rest of crew already speaks.
pub(crate) fn tools_of(ints: &[Integration]) -> Vec<McpTool> {
    ints.iter()
        .flat_map(|i| {
            i.tools.iter().map(move |t| McpTool {
                server: i.name.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t
                    .input_schema
                    .clone()
                    .unwrap_or_else(|| serde_json::json!({"type": "object"})),
            })
        })
        .collect()
}

/// Parse one manifest. `None` when it is not usable: a nameless server, no base URL, or a
/// name that could never be dialled as `server:tool`.
pub(crate) fn parse(text: &str) -> Option<Integration> {
    let mut i: Integration = serde_json::from_str(text).ok()?;
    i.name = crew_hive::agentname::slug(&i.name)?;
    if i.base_url.trim().is_empty() || i.tools.is_empty() {
        return None;
    }
    i.tools.retain(|t| !t.name.trim().is_empty());
    (!i.tools.is_empty()).then_some(i)
}

/// Every valid `.json` manifest in `dir`, by file name.
pub(crate) fn load_dir(dir: &Path) -> Vec<Integration> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
        .iter()
        .filter_map(|p| parse(&std::fs::read_to_string(p).ok()?))
        .collect()
}

/// The project directory manifests live under, with the same `CREW_PROJECT_DIR` seam every
/// other discovery path uses.
fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// User + project integrations, a project manifest replacing a user one of the same name —
/// exactly how plugin agents, skills and MCP servers already resolve.
pub(crate) fn load() -> Vec<Integration> {
    load_at(&base_dir())
}

pub(crate) fn load_at(base: &Path) -> Vec<Integration> {
    let mut all = dirs::config_dir()
        .map(|d| load_dir(&d.join("crew").join("integrations")))
        .unwrap_or_default();
    for p in load_dir(&base.join(".crew").join("integrations")) {
        match all.iter().position(|a| a.name == p.name) {
            Some(i) => all[i] = p,
            None => all.push(p),
        }
    }
    all
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
