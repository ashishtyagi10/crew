//! A swarm worker's own checklist.
//!
//! The swarm hands every worker the SAME tool surface (one MCP host, one
//! approval gate — `ApiFactory`), and that surface keeps one checklist per
//! task of the pane, which is right for the relay and wrong here: four
//! workers running at once would each write the list the others read, and
//! every worker's answer would report the others' unfinished steps. So each
//! worker's task sees the shared surface through this, which answers
//! `sys:todo` from a list of its own and passes everything else through
//! untouched.

use std::sync::Arc;

use super::Checklist;
use crate::tools::{ToolSpec, Tools};

/// `tools` for one task: behind a checklist of its own when the surface
/// keeps checklists at all, and `tools` itself when it does not, so a surface
/// without `sys:todo` is the very same surface it was.
pub fn per_task(tools: Arc<dyn Tools>) -> Arc<dyn Tools> {
    if tools.checklist().is_none() {
        return tools;
    }
    Arc::new(PerTask {
        inner: tools,
        list: Checklist::default(),
    })
}

struct PerTask {
    inner: Arc<dyn Tools>,
    list: Checklist,
}

impl Tools for PerTask {
    fn hint(&self) -> String {
        self.inner.hint()
    }

    fn call(&self, server: &str, tool: &str, args: &str) -> Result<String, String> {
        match (server, tool) {
            ("sys", "todo") => self.list.write(args),
            _ => self.inner.call(server, tool, args),
        }
    }

    fn failed(&self, server: &str, tool: &str, output: &str) -> bool {
        self.inner.failed(server, tool, output)
    }

    fn repeatable(&self, server: &str, tool: &str) -> bool {
        self.inner.repeatable(server, tool)
    }

    fn specs(&self) -> Vec<ToolSpec> {
        self.inner.specs()
    }

    fn capabilities(&self) -> Vec<String> {
        self.inner.capabilities()
    }

    fn hint_for(&self, task: &str) -> String {
        self.inner.hint_for(task)
    }

    fn specs_for(&self, task: &str) -> Vec<ToolSpec> {
        self.inner.specs_for(task)
    }

    fn note_for(&self, task: &str) -> Option<String> {
        self.inner.note_for(task)
    }

    fn checklist(&self) -> Option<&Checklist> {
        Some(&self.list)
    }
}
