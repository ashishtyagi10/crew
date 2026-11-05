//! The tools section for an agent with hands of its own (Claude Code, codex,
//! opencode): crew's tools it has no twin for, never the ones it has.
//!
//! Every hop of a thread was handed the same TOOLS section, Claude Code's
//! included: `sys:read_file`, `sys:write_file`, `sys:edit`, `sys:run`, and a
//! LOOK FIRST line sending it to `sys:grep`/`sys:glob`. It already has Read,
//! Write, Edit, Bash, Grep and Glob, so the list was a second way to do what
//! it does, in crew's syntax. Loop 8's live run showed the cost: denied its
//! own Write by the user's settings, Claude Code answered with
//! `@tool sys:write_file` — routing round the user's choice, and printing
//! crew's syntax at them. What it lacks stays: MCP servers, integrations,
//! `sys:todo`, `sys:outline`, `sys:find_tools`, the web.
//!
//! The section alone was not enough. Measured live with Write, Edit and Bash
//! denied in `.claude/settings.json`: with the twins gone from the list,
//! Claude Code asked `sys:find_tools` for "write file", was handed
//! `sys:write_file`, and wrote through it anyway — one round slower. So the
//! hop's tool surface is [`OwnHands`] too: a twin is never found, and a call
//! to one is refused with the reason, which leaves the refusal the user's.
use std::sync::Arc;

use crew_hive::tools::{todo::Checklist, ToolSpec};

use super::super::toolcall::ToolRunner;
use super::super::Adapter;

/// crew's `sys:` tools a file-reading agent has a twin for (Read, Write,
/// Edit, Bash, Grep, Glob, LS; git runs through Bash).
const TWINS: [&str; 8] = [
    "read_file",
    "write_file",
    "edit",
    "run",
    "grep",
    "glob",
    "list_dir",
    "git",
];

/// `hint` (`toolcall::hint_for`'s section) without the tools the agent has
/// itself and without LOOK FIRST, which points at two of them. Empty when
/// nothing it lacks is left — no section beats one that lists no tool.
pub(crate) fn for_own_hands(hint: &str) -> String {
    let kept: Vec<&str> = hint
        .lines()
        .filter(|l| !l.starts_with("LOOK FIRST:") && !is_twin(l))
        .collect();
    // A tool row, or the note on tools left out (they are found by name).
    let offers = |l: &&str| l.starts_with("- ") || l.contains("sys:find_tools");
    match kept.iter().any(offers) {
        true => kept.join("\n"),
        false => String::new(),
    }
}

/// Whether `line` is a tool row (`- sys:<name> — …`) for one of [`TWINS`].
fn is_twin(line: &str) -> bool {
    line.strip_prefix("- sys:")
        .and_then(|rest| rest.split_whitespace().next())
        .is_some_and(|name| TWINS.contains(&name))
}

/// What the agent's turn may call: `tools` as they are, or behind
/// [`OwnHands`] when the agent opens files itself.
pub(crate) fn fitted(agent: &dyn Adapter, tools: Arc<dyn ToolRunner>) -> Arc<dyn ToolRunner> {
    match agent.reads_files() {
        true => Arc::new(OwnHands(tools)),
        false => tools,
    }
}

/// A tool surface without crew's copies of the agent's own tools.
pub(crate) struct OwnHands(pub(crate) Arc<dyn ToolRunner>);

/// Said to an agent that calls a twin anyway: its own tool is the way, and a
/// refusal of that tool is the user's to lift, not crew's to go round.
const REFUSED: &str = "is crew's copy of a tool you have yourself \u{2014} use your own. \
     If your settings refused it, tell the user what was refused instead of going round it.";

impl ToolRunner for OwnHands {
    fn hint(&self) -> String {
        for_own_hands(&self.0.hint())
    }
    fn hint_for(&self, task: &str) -> String {
        for_own_hands(&self.0.hint_for(task))
    }
    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        if server == "sys" && TWINS.contains(&tool) {
            return Err(format!("sys:{tool} {REFUSED}"));
        }
        let out = self.0.call(server, tool, args)?;
        Ok(match (server, tool) {
            ("sys", "find_tools") => out
                .lines()
                .filter(|l| !is_twin(l))
                .collect::<Vec<_>>()
                .join("\n"),
            _ => out,
        })
    }
    fn failed(&self, server: &str, tool: &str, output: &str) -> bool {
        self.0.failed(server, tool, output)
    }
    fn repeatable(&self, server: &str, tool: &str) -> bool {
        self.0.repeatable(server, tool)
    }
    fn specs(&self) -> Vec<ToolSpec> {
        mine(self.0.specs())
    }
    fn specs_for(&self, task: &str) -> Vec<ToolSpec> {
        mine(self.0.specs_for(task))
    }
    fn capabilities(&self) -> Vec<String> {
        self.0.capabilities()
    }
    fn note_for(&self, task: &str) -> Option<String> {
        self.0.note_for(task)
    }
    fn checklist(&self) -> Option<&Checklist> {
        self.0.checklist()
    }
}

/// `specs` without the twins, so the native path offers what the text does.
fn mine(specs: Vec<ToolSpec>) -> Vec<ToolSpec> {
    let twin = |s: &ToolSpec| s.server == "sys" && TWINS.contains(&s.tool.as_str());
    specs.into_iter().filter(|s| !twin(s)).collect()
}

#[cfg(test)]
#[path = "ownhands_tests.rs"]
mod tests;
