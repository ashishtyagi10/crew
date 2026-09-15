//! What the graph is ASKED — every read a caller makes of a live `Recall`.
//!
//! Split from the facade so `mod.rs` stays the store's own story (open,
//! record, flush, switch) and this stays the reading story: the block a task
//! carries, the counts the pane says, the newest turn a pane opening quotes,
//! the files the router is told about. Same module tree, so the private
//! fields are still in reach.
use super::repeats::{self, Prior, PRIOR_FILES};
use super::{block, now_ms, query, Recall, Recalled, RECALL_CAP};

/// Files named in the router's world line. Two is a hint; the whole list
/// would be a catalogue in a prompt that is meant to be cheap.
const WORLD_FILES: usize = 2;

impl Recall {
    /// What `task` recalls — the block and its counts — or `None` when the
    /// graph has nothing to say about it.
    pub(crate) fn recalled(&self, task: &str, skip: &[String]) -> Option<Recalled> {
        if !self.on {
            return None;
        }
        block::block(&self.g, task, skip, now_ms(), RECALL_CAP)
    }

    /// The block alone, for the arms that only put it in front of a task.
    pub(crate) fn context(&self, task: &str, skip: &[String]) -> Option<String> {
        self.recalled(task, skip).map(|r| r.text)
    }

    /// The newest remembered turn whose request starts with `prefix`, and
    /// how long ago it landed — what a pane opening asks of memory.
    pub(crate) fn latest(&self, prefix: &str) -> Option<(String, u64)> {
        self.on.then(|| query::latest(&self.g, prefix)).flatten()
    }

    /// An earlier failure of `cmd` with this output. Off means no memory at
    /// all, so a silenced graph cannot claim to have seen anything.
    pub(crate) fn seen_failing(&self, cmd: &str, output: &str) -> Option<Prior> {
        self.on
            .then(|| repeats::prior(&self.g, cmd, output, PRIOR_FILES))
            .flatten()
    }

    /// The files `task` activates, strongest first — the router's world.
    pub(crate) fn files_about(&self, task: &str) -> Vec<String> {
        match self.on {
            true => query::files(&self.g, task, WORLD_FILES),
            false => Vec::new(),
        }
    }
}
