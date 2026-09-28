//! The documents a [`Client`] has told its server about, and what the
//! server says back on its own initiative. Split from `client` along the
//! line the protocol draws: `client` is request/response, this is the
//! notifications flowing both ways.
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::demux::Notification;
use crate::docsync::{digest, OpenDoc, Synced};
use crate::types::Diagnostic;
use crate::Client;

impl Client {
    pub fn is_open(&self, uri: &str) -> bool {
        self.open.contains_key(uri)
    }

    /// Tell the server about a document. The text is what the caller has,
    /// which for crew is the disk's.
    pub fn did_open(&mut self, uri: &str, language_id: &str, text: &str) -> Result<(), String> {
        self.open_at(uri, language_id, text, 1)
    }

    fn open_at(&mut self, uri: &str, lang: &str, text: &str, version: i32) -> Result<(), String> {
        self.notify(
            "textDocument/didOpen",
            json!({"textDocument": {"uri": uri, "languageId": lang, "version": version, "text": text}}),
        )?;
        self.remember(uri, version, text, lang.to_string());
        Ok(())
    }

    /// What the server now has for `uri`: `version`, and `text`'s digest.
    fn remember(&mut self, uri: &str, version: i32, text: &str, lang: String) {
        let digest = digest(text);
        let doc = OpenDoc {
            version,
            digest,
            lang,
        };
        self.open.insert(uri.to_string(), doc);
    }

    /// Replace the server's copy of an open document with `text`, whole.
    /// Whole-document changes are legal whatever sync kind the server asked
    /// for, and crew has no edit to describe anyway, only the disk's new
    /// text. A server that takes no changes gets the document closed and
    /// opened again, still at the next version.
    pub fn did_change(&mut self, uri: &str, text: &str) -> Result<(), String> {
        let doc = self
            .open
            .get(uri)
            .ok_or_else(|| format!("{uri} is not open"))?;
        let (version, lang) = (doc.version + 1, doc.lang.clone());
        if !self.doc_sync.change {
            self.did_close(uri)?;
            return self.open_at(uri, &lang, text, version);
        }
        self.notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": version}, "contentChanges": [{"text": text}]}),
        )?;
        self.remember(uri, version, text, lang);
        Ok(())
    }

    /// Say the document is saved, if the server asked to hear it, with the
    /// text if it asked for that too. crew's text always IS the saved file,
    /// and rust-analyzer only runs `cargo check` on a save.
    pub fn did_save(&mut self, uri: &str, text: &str) -> Result<(), String> {
        let Some(with_text) = self.doc_sync.save else {
            return Ok(());
        };
        let mut params = json!({"textDocument": {"uri": uri}});
        if with_text {
            params["text"] = Value::from(text);
        }
        self.notify("textDocument/didSave", params)
    }

    /// Bring the server's copy of `uri` level with `text`: open it if the
    /// server has never seen it, send the whole text and a save if it has
    /// moved on, and send nothing if it has not. A document opened here is
    /// not saved; whether a first look is worth a `cargo check` is the
    /// caller's call.
    pub fn sync(&mut self, uri: &str, language_id: &str, text: &str) -> Result<Synced, String> {
        let Some(doc) = self.open.get(uri) else {
            self.did_open(uri, language_id, text)?;
            return Ok(Synced::Opened);
        };
        if doc.digest == digest(text) {
            return Ok(Synced::Unchanged);
        }
        self.did_change(uri, text)?;
        self.did_save(uri, text)?;
        Ok(Synced::Changed)
    }

    pub fn did_close(&mut self, uri: &str) -> Result<(), String> {
        self.open.remove(uri);
        self.notify(
            "textDocument/didClose",
            json!({"textDocument": {"uri": uri}}),
        )
    }

    pub fn try_notification(&self) -> Option<Notification> {
        self.notif.try_recv().ok()
    }

    pub fn wait_notification(&self, timeout: Duration) -> Option<Notification> {
        self.notif.recv_timeout(timeout).ok()
    }

    /// Wait for the next `publishDiagnostics` about `uri`. Other files'
    /// diagnostics and every other notification are consumed on the way.
    pub fn wait_diagnostics(&self, uri: &str, timeout: Duration) -> Option<Vec<Diagnostic>> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let n = self.wait_notification(left)?;
            if n.method == "textDocument/publishDiagnostics" {
                if let Some((for_uri, list)) = Diagnostic::parse_publish(&n.params) {
                    if for_uri == uri {
                        return Some(list);
                    }
                }
            }
        }
    }

    /// The polite exit: `shutdown`, briefly waited for, then `exit`. Drop
    /// does the impolite one either way.
    pub fn shutdown(&mut self) {
        let _ = self.request("shutdown", Value::Null, Duration::from_secs(2));
        let _ = self.notify("exit", Value::Null);
    }
}
