//! How a server wants to hear that a document it has open has changed, read
//! once from its `initialize` answer, and the record of what it was last told.
//!
//! crew edits nothing itself, but the agents it serves edit files on disk
//! between two questions about them, and a server answers about the copy it
//! was sent, never the disk. So the copy is replaced whenever the disk moves
//! on, and the server says how it wants that done: `textDocumentSync` is a
//! bare kind (the old spelling) or an object naming `change` and `save`
//! separately. Both are read the way VS Code's client reads them, since that
//! reading is the one every server has been tested against.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_json::Value;

/// What a server asked to hear about the documents it has open.
///
/// The default is what an answer that says nothing means in the spec:
/// `TextDocumentSyncKind.None`, and no saves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocSync {
    /// Whether it takes `didChange`. Without it the only way to hand the
    /// server new text is to close the document and open it again.
    pub change: bool,
    /// Whether it wants `didSave`, and if so whether with the text
    /// (`Some(true)`, its `includeText`). rust-analyzer runs `cargo check`,
    /// which is where the type errors come from, only on a save.
    pub save: Option<bool>,
}

impl DocSync {
    /// The server's `textDocumentSync`, from its whole `initialize` result.
    pub fn from_initialize(result: &Value) -> Self {
        let Some(sync) = result.pointer("/capabilities/textDocumentSync") else {
            return Self::default();
        };
        // The bare kind: anything but None takes changes, and saves without
        // the text, which is how VS Code has always treated it.
        if let Some(kind) = sync.as_u64() {
            return Self {
                change: kind != 0,
                save: (kind != 0).then_some(false),
            };
        }
        let save = match sync.get("save") {
            Some(Value::Bool(true)) => Some(false),
            Some(Value::Object(o)) => Some(o.get("includeText") == Some(&Value::Bool(true))),
            _ => None,
        };
        Self {
            change: sync
                .get("change")
                .and_then(Value::as_u64)
                .is_some_and(|k| k != 0),
            save,
        }
    }
}

/// What [`crate::Client::sync`] had to tell the server to bring its copy of
/// a document level with the text it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Synced {
    /// The server had not seen the document: it was opened, at version 1.
    Opened,
    /// The text had moved on: the server was sent all of it and a save.
    Changed,
    /// The server already has this text. Nothing was sent.
    Unchanged,
}

/// One document the server has open: the version it last heard, a digest of
/// the text that came with it (enough to tell "same" from "moved on" without
/// keeping every open file twice), and the language it was opened as, which
/// a server that takes no changes has to be told again on the reopen.
#[derive(Debug, Clone)]
pub(crate) struct OpenDoc {
    pub(crate) version: i32,
    pub(crate) digest: u64,
    pub(crate) lang: String,
}

pub(crate) fn digest(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

#[cfg(test)]
#[path = "docsync_tests.rs"]
mod tests;
