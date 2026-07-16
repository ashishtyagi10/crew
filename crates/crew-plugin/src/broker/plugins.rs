//! Plugin agents: a JSON manifest turns any headless CLI into a `/crew`
//! roster agent — no recompile. Manifests live in `~/.config/crew/agents/`
//! (user) and `./.crew/agents/` (project; wins on a name collision):
//! `{"name": "aider", "command": "aider", "args": ["--message", "{}"],
//! "role": "repo-wide edits"}`. `{}` in `args` is the message placeholder
//! (appended when absent); manifests whose command isn't installed are
//! dropped by the normal probe.
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::adapter::{Adapter, CliAdapter, Normalize};

/// One parsed manifest file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Manifest {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub role: String,
}

/// A manifest-defined agent: a [`CliAdapter`] plus the manifest's role hint.
pub(crate) struct PluginAgent {
    cli: CliAdapter,
    role: String,
}

impl Adapter for PluginAgent {
    fn name(&self) -> &str {
        self.cli.name()
    }

    fn role(&self) -> &str {
        &self.role
    }

    fn probe(&self) -> bool {
        self.cli.probe()
    }

    fn call(&self, body: &str, timeout: std::time::Duration) -> Result<String, String> {
        self.cli.call(body, timeout)
    }
}

/// Build an agent from a manifest. `None` when the name or command is blank.
/// A missing `{}` placeholder is appended so the task always reaches the CLI.
pub(crate) fn from_manifest(m: Manifest) -> Option<PluginAgent> {
    // One legal-name authority (`crew_hive::agentname::slug`): the old
    // hand-rolled lowercase+hyphenate let `+`/`@` through, and such a name
    // could never be @-dialled anyway.
    let name = crew_hive::agentname::slug(&m.name)?;
    let command = m.command.trim().to_string();
    if command.is_empty() {
        return None;
    }
    let mut args = m.args;
    if !args.iter().any(|a| a.contains("{}")) {
        args.push("{}".into());
    }
    Some(PluginAgent {
        cli: CliAdapter {
            name,
            program: command,
            args,
            normalize: Normalize::Raw,
        },
        role: m.role.trim().to_string(),
    })
}

/// Every valid `.json` manifest in `dir`, sorted by file name.
pub(crate) fn load_dir(dir: &Path) -> Vec<PluginAgent> {
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
        .filter_map(|p| {
            let text = std::fs::read_to_string(p).ok()?;
            let m: Manifest = serde_json::from_str(&text).ok()?;
            from_manifest(m)
        })
        .collect()
}

/// User + project plugin agents, a project manifest replacing a user one with
/// the same name.
pub(crate) fn load() -> Vec<PluginAgent> {
    let mut all = dirs::config_dir()
        .map(|d| load_dir(&d.join("crew").join("agents")))
        .unwrap_or_default();
    for p in load_dir(Path::new(".crew/agents")) {
        match all.iter_mut().position(|a| a.name() == p.name()) {
            Some(i) => all[i] = p,
            None => all.push(p),
        }
    }
    all
}

/// Append every installed plugin agent to `agents`, skipping names the
/// roster already has (a manifest can't shadow an inbuilt agent).
pub(crate) fn append(agents: &mut Vec<Box<dyn Adapter>>) {
    for p in load() {
        let taken = agents
            .iter()
            .any(|a| a.name().eq_ignore_ascii_case(p.name()));
        if !taken && p.probe() {
            agents.push(Box::new(p));
        }
    }
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod tests;
