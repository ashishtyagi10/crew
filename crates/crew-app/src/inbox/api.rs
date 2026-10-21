//! The Gmail REST calls the inbox makes, on one worker-owned runtime.
//!
//! A page of the list is `threads.list` (ids) and then `threads.get` with
//! `format=metadata` for each id, run together: one after another they were a
//! round trip each before the first row could draw. An opened thread is one
//! `threads.get` with `format=full`; the unread count, one `labels.get INBOX`.
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
    /// The message's id, which its attachments are fetched by.
    pub id: String,
    pub from: String,
    pub date: String,
    /// What agent smith and the deadline finder read: [`Rich::whole`].
    ///
    /// [`Rich::whole`]: super::rich::Rich::whole
    pub text: String,
    pub files: Vec<String>,
    /// The message to draw (`reader`), and the parts its pictures are in.
    pub rich: super::rich::Rich,
    pub parts: Vec<super::body::Part>,
    /// Every file sent with it, to save and open.
    pub attached: Vec<super::body::Part>,
    /// What a reply to it needs: its id, subject and where answers go.
    pub reply: super::draft::Reply,
    /// How to unsubscribe from what sent it (`unsub`), when it says.
    pub unsub: Option<super::unsub::How>,
}

pub(crate) struct Gmail {
    grant: Grant,
    token_url: String,
    api: String,
    access: Option<(String, Instant)>,
    pub(super) rt: tokio::runtime::Runtime,
    pub(super) http: reqwest::Client,
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

    pub(crate) fn get(&mut self, path: &str) -> Result<Value, String> {
        Ok(self.send_all(&[(path.to_string(), None)])?.remove(0))
    }

    pub(crate) fn post(&mut self, path: &str, body: Value) -> Result<Value, String> {
        Ok(self.send_all(&[(path.to_string(), Some(body))])?.remove(0))
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
            "format=metadata&metadataHeaders=From&metadataHeaders=Subject&metadataHeaders=Date&metadataHeaders=Content-Type";
        let reqs: Vec<_> = ids
            .iter()
            .map(|id| (format!("/threads/{id}?{meta}"), None))
            .collect();
        let rows = self.send_all(&reqs)?.iter().map(wire::thread).collect();
        Ok((rows, next))
    }

    /// The nav card's reading: unread threads, and the newest unread one
    /// (1 + 10 + 40 units — a full page is about 1,000).
    pub(crate) fn peek(&mut self) -> Result<(u64, Option<Thread>), String> {
        let unread = self.unread()?;
        let q = super::query::enc("in:inbox is:unread");
        let (ids, _) = wire::thread_ids(&self.get(&format!("/threads?maxResults=1&q={q}"))?);
        let Some(id) = ids.first() else {
            return Ok((unread, None));
        };
        let meta = "format=metadata&metadataHeaders=From&metadataHeaders=Subject";
        Ok((
            unread,
            Some(wire::thread(&self.get(&format!("/threads/{id}?{meta}"))?)),
        ))
    }

    /// Every message of thread `id`, oldest first, as text.
    pub(crate) fn open(&mut self, id: &str) -> Result<Vec<Mail>, String> {
        let v = self.get(&format!("/threads/{}?format=full", super::query::enc(id)))?;
        let msgs = v["messages"].as_array().cloned().unwrap_or_default();
        Ok(msgs
            .iter()
            .map(|m| {
                let (rich, files) = super::body::text(m);
                Mail {
                    id: m["id"].as_str().unwrap_or_default().to_string(),
                    parts: super::body::pictures(m),
                    attached: super::body::files(m),
                    from: wire::header(m, "From"),
                    date: wire::header(m, "Date"),
                    text: rich.whole(),
                    files,
                    rich,
                    reply: super::draft::Reply::of(m),
                    unsub: super::unsub::of(m),
                }
            })
            .collect())
    }
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
