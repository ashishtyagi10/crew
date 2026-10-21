//! `u` in an open thread: unsubscribe, the way the mail itself offers
//! (the user, 2026-10-21: "use `u` for unsubscribe action in email detail").
//!
//! A list's mail names its way out in `List-Unsubscribe` (RFC 2369): an
//! address to mail, a page, or both. When it also says
//! `List-Unsubscribe-Post: List-Unsubscribe=One-Click` (RFC 8058), one POST
//! to its https address is the whole of it, and that is what Gmail's own
//! Unsubscribe sends. RFC 8058 counts only headers a valid DKIM signature
//! covers, so crew acts for you only on mail Gmail found `dkim=pass`; any
//! other mail can only have its page opened in the browser. Acting needs a
//! second `u`, since a mail sent from your account cannot be taken back.
use serde_json::{json, Value};

use super::api::Gmail;
use super::draft::{raw, Answer};
use super::worker::Ask;
use super::InboxPane;

/// A way out, best first.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum How {
    /// RFC 8058: one POST, nothing sent from the account.
    OneClick(String),
    /// A mail to the list's own address, sent from the account.
    Mail {
        to: String,
        subject: String,
        body: String,
    },
    /// A page to finish on, in the browser.
    Web(String),
}

/// What a message's headers offer: the best way, after the signature check.
pub(crate) fn of(msg: &Value) -> Option<How> {
    let heads = msg["payload"]["headers"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let all = |name: &str| -> Vec<String> {
        heads
            .iter()
            .filter(|h| {
                h["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
            .filter_map(|h| h["value"].as_str().map(str::to_string))
            .collect()
    };
    let signed = all("Authentication-Results")
        .iter()
        .any(|v| v.to_ascii_lowercase().contains("dkim=pass"));
    let one_click = all("List-Unsubscribe-Post").iter().any(|v| {
        v.replace(' ', "")
            .eq_ignore_ascii_case("List-Unsubscribe=One-Click")
    });
    best(&all("List-Unsubscribe").join(","), one_click, signed)
}

/// The best way among `List-Unsubscribe`'s `<...>` addresses.
pub(crate) fn best(list: &str, one_click: bool, signed: bool) -> Option<How> {
    let addrs: Vec<&str> = list
        .split('<')
        .skip(1)
        .filter_map(|a| a.split_once('>').map(|(a, _)| a.trim()))
        .collect();
    let web = addrs
        .iter()
        .find(|a| a.starts_with("https://") || a.starts_with("http://"));
    let safe = web.filter(|a| a.starts_with("https://") && public(a));
    let mail = addrs.iter().find_map(|a| a.strip_prefix("mailto:"));
    match (signed, one_click, safe, mail) {
        (true, true, Some(url), _) => Some(How::OneClick(url.to_string())),
        (true, _, _, Some(m)) => Some(mailto(m)),
        _ => web.map(|w| How::Web(w.to_string())),
    }
}

/// A host on the internet, not this machine or the local network by number.
fn public(url: &str) -> bool {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit('@').next().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    host.contains('.') && !host.ends_with(".local") && host.parse::<std::net::IpAddr>().is_err()
}

/// `addr?subject=..&body=..`, decoded.
fn mailto(m: &str) -> How {
    let (to, query) = m.split_once('?').unwrap_or((m, ""));
    let field = |k: &str| {
        query
            .split('&')
            .find_map(|kv| {
                kv.split_once('=')
                    .filter(|(n, _)| n.eq_ignore_ascii_case(k))
            })
            .map(|(_, v)| decode(v))
    };
    How::Mail {
        to: decode(to),
        subject: field("subject").unwrap_or_else(|| "unsubscribe".into()),
        body: field("body").unwrap_or_else(|| "unsubscribe".into()),
    }
}

/// Percent-decoding, leniently: a `%` not followed by two hex digits stays.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = s
            .get(i + 1..i + 3)
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        let (byte, step) = match (b[i], hex) {
            (b'%', Some(byte)) => (byte, 3),
            (c, _) => (c, 1),
        };
        out.push(byte);
        i += step;
    }
    String::from_utf8_lossy(&out).into_owned()
}

impl InboxPane {
    /// `u` in the reader: what to say. The first press says what would
    /// happen; a second, with no other key between, does it.
    pub(crate) fn unsubscribe(&mut self) -> String {
        let Some(r) = self.reading.as_mut() else {
            return String::new();
        };
        let Some(mails) = r.mails.as_deref() else {
            return "still opening the mail\u{2026}".into();
        };
        let Some((m, how)) = mails.iter().rev().find_map(|m| Some((m, m.unsub.clone()?))) else {
            return "this mail gives no way to unsubscribe".into();
        };
        let who = super::wire::sender_name(&m.from);
        if let How::Web(url) = &how {
            crate::linkclick::open_external(url);
            return format!("{who}\u{2019}s unsubscribe page is open in your browser");
        }
        if !std::mem::replace(&mut r.unsub_armed, true) {
            let what = match &how {
                How::OneClick(_) => "one click, nothing is sent from your account".to_string(),
                How::Mail { to, .. } => format!("crew mails {to} from your account"),
                How::Web(_) => String::new(),
            };
            return format!("u again unsubscribes from {who} \u{b7} {what}");
        }
        r.unsub_armed = false;
        self.ask(Ask::Unsubscribe {
            how,
            who: who.clone(),
        });
        format!("unsubscribing from {who}\u{2026}")
    }
}

impl Gmail {
    /// Carry out a way out (never [`How::Web`], which the browser opens).
    pub(crate) fn unsubscribe(&mut self, how: &How) -> Result<(), String> {
        match how {
            How::OneClick(url) => {
                let host = url.split('/').nth(2).unwrap_or(url).to_string();
                let req = self
                    .http
                    .post(url)
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body("List-Unsubscribe=One-Click")
                    .timeout(std::time::Duration::from_secs(20));
                // Sent inside the runtime: a timeout's timer needs its reactor.
                let res = self
                    .rt
                    .block_on(async move { req.send().await })
                    .map_err(|_| format!("could not reach {host} to unsubscribe"))?;
                match res.status().is_success() {
                    true => Ok(()),
                    false => Err(format!("{host} refused the unsubscribe ({})", res.status())),
                }
            }
            How::Mail { to, subject, body } => {
                let a = Answer {
                    to: to.clone(),
                    subject: subject.clone(),
                    ..Answer::default()
                };
                let sent = self.post("/messages/send", json!({ "raw": raw(&a, body) }));
                sent.map(|_| ())
            }
            How::Web(_) => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "unsub_tests.rs"]
mod tests;
