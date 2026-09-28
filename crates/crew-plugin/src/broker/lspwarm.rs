//! The language server starts when a task first writes a file, not when the
//! task ends.
//!
//! The end of a task asks the language servers about the files it changed
//! (`taskdiag`), and until then nothing had started one, so that first ask
//! paid the server's whole cold start. Measured on the crew repo: a task that
//! added one doc comment had its file summary out at +42.5 s, and its diff and
//! verdict at +54.9 s — twelve seconds of rust-analyzer indexing. A task's
//! first `sys:write_file` or `sys:edit` says which language it is about to
//! need, and it comes well before the end (the model still has to answer), so
//! that is when the server is started, on a thread of its own: by the time the
//! task ends the index has had those seconds.
//!
//! What it costs: the server the task's end would have started anyway, a
//! little earlier, and only in tasks that write a file some installed server
//! serves. Reads start nothing, and neither does a write that failed.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Starts whatever serves a file. Called on the tool call's own thread, so it
/// must not block.
pub(crate) trait Warmer: Send + Sync {
    fn warm(&self, path: &Path);
}

/// The session's language servers, started on a thread of their own: a start
/// holds the host's lock through the spawn and `initialize`, and the write
/// that asked for it must not wait on either.
struct HostWarmer(Arc<Mutex<crate::lsp::LspHost>>);

impl Warmer for HostWarmer {
    fn warm(&self, path: &Path) {
        let (lsp, path) = (Arc::clone(&self.0), path.to_path_buf());
        std::thread::spawn(move || lsp.lock().unwrap_or_else(|e| e.into_inner()).warm(&path));
    }
}

/// One task's record of the languages it has already warmed, so a task that
/// edits twenty Rust files starts one thread, not twenty.
pub(crate) struct WarmOnWrite {
    warmer: Arc<dyn Warmer>,
    asked: Mutex<BTreeSet<&'static str>>,
}

impl WarmOnWrite {
    /// A record over `warmer`, with nothing asked yet.
    pub(crate) fn new(warmer: Arc<dyn Warmer>) -> Arc<Self> {
        Arc::new(Self {
            warmer,
            asked: Mutex::default(),
        })
    }

    /// A record that starts the session's own language servers.
    pub(crate) fn over(lsp: &Arc<Mutex<crate::lsp::LspHost>>) -> Arc<Self> {
        Self::new(Arc::new(HostWarmer(Arc::clone(lsp))))
    }

    /// The same warmer with nothing asked: the next task's record. Per task
    /// rather than per session, because the host drops a client whose request
    /// failed and a pane can move to another project; asking again once a
    /// task costs a thread and a lock that finds the server running.
    pub(crate) fn fresh(&self) -> Arc<Self> {
        Self::new(Arc::clone(&self.warmer))
    }

    /// A `sys` tool call has succeeded: the first write of each language this
    /// task makes starts its server.
    pub(crate) fn wrote(&self, tool: &str, args: &str) {
        if !matches!(tool, "write_file" | "edit") {
            return;
        }
        let Some(path) = path_arg(args) else {
            return;
        };
        let Some(lang) = crew_lsp::servers::lang_of(&path) else {
            return;
        };
        let first = self
            .asked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(lang);
        if first {
            self.warmer.warm(&path);
        }
    }
}

/// The `path` a write named, as the tool read it.
pub(super) fn path_arg(args: &str) -> Option<PathBuf> {
    let v: serde_json::Value = serde_json::from_str(args).ok()?;
    v.get("path")?.as_str().map(PathBuf::from)
}

#[cfg(test)]
#[path = "lspwarm_tests.rs"]
mod tests;
