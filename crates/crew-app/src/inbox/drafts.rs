//! Gmail drafts, on the worker's side: the box saved as one (⌘S, created or
//! brought up to date), and a thread's drafts found when it opens, so the
//! pane can put one back in the box (`draftopen`). A draft's files are
//! fetched to a folder of crew's own as it opens, so the box holds them as
//! files of yours: saved again or sent, they go with it.
use serde_json::{json, Value};

use super::api::{Gmail, Mail};
use super::attach::Attached;
use super::outgoing::{self, Outgoing};

#[path = "draftopen.rs"]
mod open;
pub(crate) use open::SAVE;

/// A message of a thread that is a draft: its id among Gmail's drafts, and
/// its files, on disk.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Draft {
    pub id: String,
    pub files: Vec<Attached>,
}

/// Save `o` as a draft: a new one, or the draft it is (`o.draft`), brought
/// up to what it says now. The draft's id.
pub(crate) fn save(g: &mut Gmail, o: &Outgoing) -> Result<String, String> {
    let m = outgoing::build(g, o)?;
    let v = match &o.draft {
        Some(id) => g.upload(true, &format!("/drafts/{id}"), m, Some(meta(Some(id), o)))?,
        None => {
            let meta = o.thread_id.as_ref().map(|_| meta(None, o));
            g.upload(false, "/drafts", m, meta)?
        }
    };
    match v["id"].as_str() {
        Some(id) if !id.is_empty() => Ok(id.to_string()),
        _ => Err("Gmail kept the draft but did not say as what".into()),
    }
}

/// The draft resource beside the message: its id, and its thread.
fn meta(id: Option<&String>, o: &Outgoing) -> Value {
    let mut v = json!({});
    if let Some(id) = id {
        v["id"] = json!(id);
    }
    if let Some(t) = &o.thread_id {
        v["message"] = json!({ "threadId": t });
    }
    v
}

/// The drafts among `mails` (marked by `DRAFT`, `Draft::default()`), named
/// by their draft ids, their files fetched. A draft Gmail does not list, or
/// whose files will not come, stays a mail to read.
pub(crate) fn find(g: &mut Gmail, mails: &mut [Mail]) {
    if mails.iter().all(|m| m.draft.is_none()) {
        return;
    }
    let listed = g.get("/drafts?maxResults=500").unwrap_or_default();
    let ids = ids(&listed);
    for m in mails.iter_mut().filter(|m| m.draft.is_some()) {
        let id = ids
            .iter()
            .find(|(_, msg)| *msg == m.id)
            .map(|(d, _)| d.clone());
        m.draft = id.and_then(|id| {
            files(g, m, &id)
                .ok()
                .map(|files| Draft { id, files })
                .map(Box::new)
        });
    }
}

/// `drafts.list`'s answer as (draft id, message id) pairs.
pub(crate) fn ids(v: &Value) -> Vec<(String, String)> {
    let s = |v: &Value| v.as_str().unwrap_or_default().to_string();
    v["drafts"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|d| (s(&d["id"]), s(&d["message"]["id"])))
                .collect()
        })
        .unwrap_or_default()
}

/// Draft `id`'s files, written under crew's own folder for it.
fn files(g: &mut Gmail, m: &Mail, id: &str) -> Result<Vec<Attached>, String> {
    let dir = std::env::temp_dir()
        .join("crew-drafts")
        .join(super::save::safe_name(id));
    let mut out = Vec::new();
    for p in &m.attached {
        let bytes = match (p.data.as_deref().and_then(super::body::bytes), &p.att) {
            (Some(b), _) => b,
            (None, Some(a)) => g.attachment(&m.id, a)?,
            (None, None) => continue,
        };
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(super::save::safe_name(&p.name));
        std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        out.push(Attached {
            path,
            name: p.name.clone(),
            size: bytes.len() as u64,
        });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "drafts_tests.rs"]
mod tests;
