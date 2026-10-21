//! `f`: the open thread's newest mail forwarded, as Gmail forwards one. A
//! note of yours on top, then `---------- Forwarded message ---------`, the
//! sender, date, subject and recipients, and the mail itself: its own HTML
//! (with the pictures it draws by `cid:`) and its plain words, and every
//! file it carried. The bytes of pictures and files are fetched by the
//! worker as it sends, never before.
use serde_json::Value;

use super::api::Mail;
use super::body::Part;

/// The most a forward's files may come to: Gmail's own limit.
pub(crate) const MOST: u64 = 25 * 1024 * 1024;

/// A message's HTML and plain words, as they were sent.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Source {
    pub html: Option<String>,
    pub plain: Option<String>,
}

impl Source {
    pub(crate) fn of(msg: &Value) -> Self {
        let find = |mime| super::body::find(&msg["payload"], mime).map(|(t, _)| t);
        Self {
            html: find("text/html"),
            plain: find("text/plain"),
        }
    }
}

/// The mail being forwarded, as the box keeps it until it goes.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Forwarded {
    /// The message's id, which its files are fetched by.
    pub msg: String,
    pub from: String,
    pub date: String,
    pub subject: String,
    pub to: String,
    pub cc: String,
    pub source: Source,
    /// Its words as the reader has them, when it sent no plain part.
    pub text: String,
    pub files: Vec<Part>,
    /// The pictures its HTML names by `cid:`.
    pub inline: Vec<Part>,
}

impl Forwarded {
    pub(crate) fn of(m: &Mail) -> Self {
        let html = m.reply.source.html.as_deref().unwrap_or("");
        let inline: Vec<Part> = m
            .parts
            .iter()
            .filter(|p| {
                p.cid
                    .as_ref()
                    .is_some_and(|c| html.contains(&format!("cid:{c}")))
            })
            .cloned()
            .collect();
        let same = |a: &Part, b: &Part| {
            (a.att.is_some() && a.att == b.att) || (a.data.is_some() && a.data == b.data)
        };
        let files = m
            .attached
            .iter()
            .filter(|f| !inline.iter().any(|p| same(p, f)))
            .cloned()
            .collect();
        Self {
            msg: m.id.clone(),
            from: m.from.clone(),
            date: m.date.clone(),
            subject: m.reply.subject.clone(),
            to: m.reply.to.clone(),
            cc: m.reply.cc.clone(),
            source: m.reply.source.clone(),
            text: m.text.clone(),
            files,
            inline,
        }
    }

    /// What its files and pictures come to, in bytes.
    pub(crate) fn size(&self) -> u64 {
        self.files.iter().chain(&self.inline).map(|p| p.size).sum()
    }

    /// The header lines Gmail puts over a forwarded mail.
    pub(crate) fn lines(&self) -> Vec<(&'static str, &str)> {
        let all = [
            ("From", self.from.as_str()),
            ("Date", self.date.as_str()),
            ("Subject", self.subject.as_str()),
            ("To", self.to.as_str()),
            ("Cc", self.cc.as_str()),
        ];
        all.into_iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .collect()
    }
}

/// `Fwd:` once on the subject.
pub(crate) fn subject(s: &str) -> String {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    match lower.starts_with("fwd:") || lower.starts_with("fw:") {
        true => s.to_string(),
        false => format!("Fwd: {s}"),
    }
}

#[cfg(test)]
#[path = "forward_tests.rs"]
pub(crate) mod tests;
