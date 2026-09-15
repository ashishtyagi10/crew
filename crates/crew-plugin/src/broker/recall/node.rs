//! The two record types the graph is made of, and the one-line JSON each is
//! written as.
//!
//! Deliberately small: a node is a NAME under a kind, an edge is a pair under
//! a relation, and both carry a hit count and a last-seen stamp so ranking has
//! something to work with. Anything richer (embeddings, a schema per kind)
//! would need a store this does not have; anything poorer could not answer
//! "what do I already know about X".
use std::fmt;

/// What a node stands for. `Topic` is a word the conversation kept using,
/// `File` a path that came up, `Turn` one exchange — the only kind that
/// carries text, because it is the only kind worth quoting back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    Topic,
    File,
    Turn,
}

impl Kind {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Kind::Topic => "topic",
            Kind::File => "file",
            Kind::Turn => "turn",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "topic" => Some(Kind::Topic),
            "file" => Some(Kind::File),
            "turn" => Some(Kind::Turn),
            _ => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// How two nodes are related. `Mentions` is a turn to a topic or a file,
/// `Then` one turn to the next (the session's spine), `With` two topics that
/// came up together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Rel {
    Mentions,
    Then,
    With,
}

impl Rel {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Rel::Mentions => "mentions",
            Rel::Then => "then",
            Rel::With => "with",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "mentions" => Some(Rel::Mentions),
            "then" => Some(Rel::Then),
            "with" => Some(Rel::With),
            _ => None,
        }
    }
}

/// A node's index in [`super::Graph::nodes`]. Stable for the life of a file:
/// the log is append-only and compaction rewrites both sides together.
pub(crate) type NodeId = u32;

/// One remembered thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub kind: Kind,
    /// The match key — lowercased, trimmed. Two spellings of one topic are
    /// one node, which is the whole point of having nodes at all.
    pub key: String,
    /// What to show: a turn's text, a file's path as typed, a topic's word.
    pub text: String,
    /// Times this node was seen. Ranking's cheap stand-in for importance.
    pub hits: u32,
    /// Unix-epoch milliseconds of the most recent sighting.
    pub last_ms: u64,
}

/// One remembered connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub rel: Rel,
    pub weight: u32,
}

#[cfg(test)]
#[path = "node_tests.rs"]
mod tests;
