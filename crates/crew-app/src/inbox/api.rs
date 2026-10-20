//! The Gmail REST calls the inbox makes, on one worker-owned runtime.
//!
//! A page of the list is `threads.list` (ids) and then `threads.get` with
//! `format=metadata` for each id, run together: twenty-five threads one after
//! another were twenty-five round trips before the first row could draw. An
//! opened thread is one `threads.get` with `format=full`. The unread count is
//! `labels.get INBOX` — one call, the glance's whole cost.
//!
//! The access token lives here, an hour at a time, renewed from the grant's
//! refresh token a minute before it lapses or when Gmail answers 401.
use std::time::{Duration, Instant};

use serde_json::Value;

use super::grant::Grant;
use super::wire::{self, Thread};

pub(crate) const API: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
/// Rows a page asks for.
pub(crate) const PAGE: u32 = 25;

/// One message of an opened thread, as the reader shows it.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Mail {
    pub from: String,
    pub date: String,
    pub text: String,
    pub files: Vec<String>,
    /// What a reply to it needs: its id, subject and where answers go.
    pub reply: super::draft::Reply,
}

pub(crate) struct Gmail {
    grant: Grant,
    token_url: String,
    api: String,
    access: Option<(String, Instant)>,
    rt: tokio::runtime::Runtime,
    http: reqwest::Client,
}

impl Gmail {
    pub(crate) fn new(grant: Grant) -> Result<Self, String> {
        Self::at(grant, super::signin::TOKEN_URL, API)
    }

    /// Against other endpoints: the tests' stub servers.
    pub(crate) fn at(grant: Grant, token_url: &str, api: &str) -> Result<Self, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "could not start the http runtime".to_string())?;
        Ok(Self {
            grant,
            token_url: token_url.into(),
            api: api.into(),
            access: None,
            rt,
            http: reqwest::Client::new(),
        })
    }

    /// The access token, renewed when it is within a minute of lapsing.
    fn token(&mut self) -> Result<String, String> {
        if let Some((t, until)) = &self.access {
            if Instant::now() < *until {
                return Ok(t.clone());
            }
        }
        let (t, life) = super::signin::refresh(&self.token_url, &self.grant)?;
        let until = Instant::now() + Duration::from_secs(life.saturating_sub(60));
        self.access = Some((t.clone(), until));
        Ok(t)
    }

    /// Send each request (a path under the API root, and a body to POST) at
    /// once, in order; one 401 renews the token and tries once more.
    fn send_all(&mut self, reqs: &[(String, Option<Value>)]) -> Result<Vec<Value>, String> {
        for attempt in 0..2 {
            let token = self.token()?;
            let reqs: Vec<_> = reqs
                .iter()
                .map(|(p, b)| (format!("{}{p}", self.api), b.clone()))
                .collect();
            let http = self.http.clone();
            match self.rt.block_on(super::fetch::all(http, token, reqs)) {
                Err(super::fetch::Fetch::Expired) if attempt == 0 => self.access = None,
                Err(super::fetch::Fetch::Expired) => {
                    return Err("Gmail refused the sign-in (401)".into())
                }
                Err(super::fetch::Fetch::Failed(e)) => return Err(e),
                Ok(values) => return Ok(values),
            }
        }
        unreachable!("the second attempt always returns")
    }

    fn get(&mut self, path: &str) -> Result<Value, String> {
        Ok(self.send_all(&[(path.to_string(), None)])?.remove(0))
    }

    pub(crate) fn post(&mut self, path: &str, body: Value) -> Result<Value, String> {
        Ok(self.send_all(&[(path.to_string(), Some(body))])?.remove(0))
    }

    /// Add and take away labels on a whole thread: archive is taking `INBOX`,
    /// read is taking `UNREAD`, a star is adding `STARRED`.
    pub(crate) fn relabel(
        &mut self,
        id: &str,
        add: &[&str],
        remove: &[&str],
    ) -> Result<(), String> {
        let body = serde_json::json!({"addLabelIds": add, "removeLabelIds": remove});
        let path = format!("/threads/{}/modify", super::query::enc(id));
        self.post(&path, body).map(|_| ())
    }

    /// The account's address, and the mailbox's current history record: it
    /// moves when anything lands, is read or is filed, so an unchanged one
    /// means nothing to re-list.
    pub(crate) fn profile(&mut self) -> Result<(String, String), String> {
        let v = self.get("/profile")?;
        let field = |k: &str| v[k].as_str().unwrap_or_default().to_string();
        Ok((field("emailAddress"), field("historyId")))
    }

    /// Unread threads in the inbox.
    pub(crate) fn unread(&mut self) -> Result<u64, String> {
        Ok(wire::unread_threads(&self.get("/labels/INBOX")?))
    }

    /// One page of threads matching `query` (Gmail's own search syntax; the
    /// empty query is the inbox), and the token for the next.
    pub(crate) fn page(
        &mut self,
        query: &str,
        from: Option<&str>,
    ) -> Result<(Vec<Thread>, Option<String>), String> {
        let q = match query.trim() {
            "" => "in:inbox".to_string(),
            q => q.to_string(),
        };
        let mut list = format!("/threads?maxResults={PAGE}&q={}", super::query::enc(&q));
        if let Some(p) = from {
            list.push_str(&format!("&pageToken={}", super::query::enc(p)));
        }
        let (ids, next) = wire::thread_ids(&self.get(&list)?);
        let meta =
            "format=metadata&metadataHeaders=From&metadataHeaders=Subject&metadataHeaders=Date";
        let reqs: Vec<_> = ids
            .iter()
            .map(|id| (format!("/threads/{id}?{meta}"), None))
            .collect();
        let rows = self.send_all(&reqs)?.iter().map(wire::thread).collect();
        Ok((rows, next))
    }

    /// Every message of thread `id`, oldest first, as text.
    pub(crate) fn open(&mut self, id: &str) -> Result<Vec<Mail>, String> {
        let v = self.get(&format!("/threads/{}?format=full", super::query::enc(id)))?;
        let msgs = v["messages"].as_array().cloned().unwrap_or_default();
        Ok(msgs
            .iter()
            .map(|m| {
                let (text, files) = super::body::text(m);
                Mail {
                    from: wire::header(m, "From"),
                    date: wire::header(m, "Date"),
                    text,
                    files,
                    reply: super::draft::Reply::of(m),
                }
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
