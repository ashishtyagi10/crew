//! A mail the worker builds before it sends: a forward, or a reply that
//! carries files. Your files are read from disk and a forwarded mail's are
//! fetched from Gmail here, off the window's thread, then the whole mail
//! goes up through Gmail's upload address (`upload`), in its thread when it
//! is a reply.
use super::api::Gmail;
use super::attach::{self, Attached};
use super::forward::Forwarded;
use super::mime::{self, Node};

pub(crate) struct Outgoing {
    /// To, Cc, Bcc, Subject and the threading headers.
    pub head: Vec<String>,
    /// The thread a reply goes in; a forward starts its own.
    pub thread_id: Option<String>,
    pub words: String,
    pub forward: Option<Box<Forwarded>>,
    pub files: Vec<Attached>,
    /// Who it goes to, for the line that says it went.
    pub to: String,
}

/// Build `o` and send it.
pub(crate) fn send(g: &mut Gmail, o: &Outgoing) -> Result<(), String> {
    let mut mine = Vec::new();
    for f in &o.files {
        let bytes =
            std::fs::read(&f.path).map_err(|e| format!("could not read {}: {e}", f.name))?;
        mine.push(Node::file(attach::mime_of(&f.name), &f.name, bytes));
    }
    let tree = match &o.forward {
        Some(of) => {
            let carried = super::forwardmail::fetch(g, of)?;
            super::forwardmail::tree(&o.words, of, carried, mine)
        }
        None => {
            let mut parts = vec![Node::text("plain", &o.words)];
            parts.extend(mine);
            Node::multi("mixed", parts)
        }
    };
    let m = mime::message(&o.head, &tree);
    g.upload_send(m.into_bytes(), o.thread_id.as_deref())
}

#[cfg(test)]
#[path = "outgoing_tests.rs"]
mod tests;
