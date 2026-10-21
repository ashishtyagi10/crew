//! Gmail's REST answers, read into the few fields the inbox draws.
//!
//! The pane lists threads, as Gmail's own inbox does: `threads.list` names
//! them, and `threads.get` with `format=metadata` gives each one's messages
//! with only the `From`, `Subject` and `Date` headers, never a body. A body is
//! fetched when a thread is opened (`body`). Everything is read leniently: a
//! field Gmail leaves out is empty, not an error, because a row with no
//! subject is still a row.
use serde_json::Value;

/// One inbox row: the thread's newest message, and what the thread holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Thread {
    pub id: String,
    /// The newest message's sender, as written (`Ann Lee <ann@x.org>`).
    pub from: String,
    pub subject: String,
    pub snippet: String,
    /// Any message in the thread still carries `UNREAD`.
    pub unread: bool,
    /// Any message in the thread carries `STARRED`.
    pub starred: bool,
    /// The newest message's `internalDate`, ms since the epoch.
    pub at_ms: i64,
    /// Messages in the thread.
    pub count: usize,
    /// Who wrote each message but a draft, oldest first: what the row's
    /// "Ann, Bo, me" is made from (`who`).
    pub senders: Vec<Sender>,
    /// A message is a draft not yet sent (`DRAFT`).
    pub draft: bool,
    /// A message is `multipart/mixed`: something is attached.
    pub attached: bool,
}

/// One message's `From`, and whether it carries `SENT` (the account's own).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Sender {
    pub from: String,
    pub sent: bool,
}

/// `threads.list`: the thread ids in the order Gmail gave them, and the
/// token for the next page.
pub(crate) fn thread_ids(v: &Value) -> (Vec<String>, Option<String>) {
    let ids = v["threads"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| t["id"].as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    (ids, v["nextPageToken"].as_str().map(str::to_string))
}

/// `threads.get?format=metadata`: the row for that thread.
pub(crate) fn thread(v: &Value) -> Thread {
    let msgs = v["messages"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let newest = msgs
        .iter()
        .max_by_key(|m| date_of(m))
        .cloned()
        .unwrap_or(Value::Null);
    let unread = msgs.iter().any(|m| has_label(m, "UNREAD"));
    let first = msgs.first().unwrap_or(&Value::Null);
    let mut written: Vec<&Value> = msgs.iter().filter(|m| !has_label(m, "DRAFT")).collect();
    written.sort_by_key(|m| date_of(m));
    Thread {
        id: v["id"].as_str().unwrap_or_default().to_string(),
        from: header(&newest, "From"),
        // A reply's subject is "Re: …"; the thread is named by its first.
        subject: header(first, "Subject"),
        snippet: unescape(newest["snippet"].as_str().unwrap_or_default()),
        unread,
        starred: msgs.iter().any(|m| has_label(m, "STARRED")),
        at_ms: date_of(&newest),
        count: msgs.len(),
        senders: written
            .iter()
            .map(|m| Sender {
                from: header(m, "From"),
                sent: has_label(m, "SENT"),
            })
            .collect(),
        draft: msgs.iter().any(|m| has_label(m, "DRAFT")),
        attached: msgs.iter().any(mixed),
    }
}

/// Whether a message is `multipart/mixed`, by its `Content-Type` header or
/// the payload's own type: the shape a mail with a file attached takes.
fn mixed(msg: &Value) -> bool {
    let kind = match msg["payload"]["mimeType"].as_str() {
        Some(t) => t.to_string(),
        None => header(msg, "Content-Type"),
    };
    kind.trim()
        .to_ascii_lowercase()
        .starts_with("multipart/mixed")
}

/// `labels.get` on `INBOX`: the threads in it still unread.
pub(crate) fn unread_threads(v: &Value) -> u64 {
    v["threadsUnread"].as_u64().unwrap_or(0)
}

/// A message's header by name, case-insensitively, as Gmail sends it.
pub(crate) fn header(msg: &Value, name: &str) -> String {
    msg["payload"]["headers"]
        .as_array()
        .and_then(|hs| {
            hs.iter().find(|h| {
                h["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
        })
        .and_then(|h| h["value"].as_str())
        .unwrap_or_default()
        .to_string()
}

fn has_label(msg: &Value, label: &str) -> bool {
    msg["labelIds"]
        .as_array()
        .is_some_and(|ls| ls.iter().any(|l| l.as_str() == Some(label)))
}

/// `internalDate` is a string of milliseconds.
fn date_of(msg: &Value) -> i64 {
    msg["internalDate"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Gmail's snippets are HTML-escaped (`&#39;`, `&amp;`): the five entities it
/// uses, back to their characters.
pub(crate) fn unescape(s: &str) -> String {
    s.replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// The display name in a `From` header: `Ann Lee <ann@x.org>` → `Ann Lee`;
/// a bare address stays an address; quotes come off.
pub(crate) fn sender_name(from: &str) -> String {
    let name = from
        .split('<')
        .next()
        .unwrap_or(from)
        .trim()
        .trim_matches('"');
    match name.is_empty() {
        true => from.trim().trim_matches(['<', '>']).to_string(),
        false => name.to_string(),
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
