//! Corrections, seen and forgotten one at a time.
//!
//! A correction is the one recollection that leads every recall it reaches
//! (`query::turns` sorts it first) and reads as an order: "ruled out
//! before". It was also the one thing in the graph the user could neither
//! read nor take back, so a line caught by mistake, or a "don't" that
//! stopped being true, steered every later task on those files until four
//! hundred turns pushed it out.
use std::collections::HashSet;

use super::node::{Kind, NodeId};
use super::ruled::RULED;
use super::{store, Recall};

impl Recall {
    /// What is ruled out, newest first: `(what, when)`.
    pub(crate) fn ruled(&self) -> Vec<(String, u64)> {
        self.ruled_ids()
            .into_iter()
            .filter_map(|id| self.g.node(id))
            .map(|n| (n.text[RULED.len()..].to_string(), n.last_ms))
            .collect()
    }

    /// Forget the `nth` correction of [`Recall::ruled`] (1-based) and say
    /// which it was. The log is rewritten whole, as at a compaction: a node
    /// leaving renumbers the rest, and an appended "forget" line would have
    /// every older build of crew replay the correction back in.
    pub(crate) fn forget_ruled(&mut self, nth: usize) -> Option<String> {
        let id = *self.ruled_ids().get(nth.checked_sub(1)?)?;
        let what = self.g.node(id)?.text[RULED.len()..].to_string();
        self.g.drop_nodes(&HashSet::from([id]));
        self.prev = None; // ids were renumbered
        store::rewrite(&self.path, &self.g);
        Some(what)
    }

    fn ruled_ids(&self) -> Vec<NodeId> {
        if !self.on {
            return Vec::new();
        }
        let mut ids: Vec<(NodeId, u64)> = self
            .g
            .nodes()
            .filter(|(_, n)| n.kind == Kind::Turn && n.text.starts_with(RULED))
            .map(|(id, n)| (id, n.last_ms))
            .collect();
        ids.sort_by_key(|(id, ms)| (std::cmp::Reverse(*ms), std::cmp::Reverse(*id)));
        ids.into_iter().map(|(id, _)| id).collect()
    }
}

#[cfg(test)]
#[path = "forget_tests.rs"]
mod tests;
