//! A reply as Gmail takes it, in its thread: whom it answers and the
//! message itself.
//!
//! Gmail files a message into a thread when three things agree: its
//! `threadId`, `In-Reply-To` and `References` naming the message answered
//! (RFC 2822), and a subject that matches ("Gmail API: Managing threads").
//! crew writes those headers itself from the thread; the words are yours,
//! or agent smith's draft of them in the box, and sending is yours.
use base64::Engine;
use serde_json::Value;

use super::addr;
use super::api::{Gmail, Mail};
use super::wire::header;

/// What a reply to one message, or its forward, needs from it.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Reply {
    pub subject: String,
    pub message_id: String,
    pub reply_to: String,
    pub references: String,
    /// Who else it went to, for reply all and a forward's header.
    pub to: String,
    pub cc: String,
    /// Its own HTML and plain words, as a forward carries them.
    pub source: super::forward::Source,
}

impl Reply {
    pub(crate) fn of(msg: &Value) -> Self {
        Self {
            subject: header(msg, "Subject"),
            message_id: header(msg, "Message-ID"),
            reply_to: header(msg, "Reply-To"),
            references: header(msg, "References"),
            to: header(msg, "To"),
            cc: header(msg, "Cc"),
            source: super::forward::Source::of(msg),
        }
    }
}

/// Everything a reply needs but its words, taken when the box opens.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Answer {
    pub thread_id: String,
    pub to: String,
    pub cc: String,
    pub subject: String,
    pub in_reply_to: String,
    pub references: String,
}

/// The reply to `mails`, from `me`: to the last message someone else sent
/// (its `Reply-To`, else its `From`), on the thread's subject with `Re:`.
pub(crate) fn answer(thread_id: &str, mails: &[Mail], me: &str) -> Option<Answer> {
    let last = answered(mails, me)?;
    let to = match last.reply.reply_to.trim() {
        "" => last.from.trim(),
        r => r,
    };
    let first = mails.first().map_or("", |m| m.reply.subject.trim());
    let subject = match first
        .get(..3)
        .is_some_and(|p| p.eq_ignore_ascii_case("re:"))
    {
        true => first.to_string(),
        false => format!("Re: {first}"),
    };
    let id = last.reply.message_id.trim();
    Some(Answer {
        thread_id: thread_id.into(),
        to: to.into(),
        cc: String::new(),
        subject,
        in_reply_to: id.into(),
        references: format!("{} {id}", last.reply.references.trim())
            .trim()
            .to_string(),
    })
}

/// The reply to everyone on the message answered, as Gmail's reply all has
/// it: its sender and the rest of its `To` in To, its `Cc` in Cc, and never
/// `me`. Every reply crew writes is this one.
pub(crate) fn answer_all(thread_id: &str, mails: &[Mail], me: &str) -> Option<Answer> {
    let one = answer(thread_id, mails, me)?;
    let last = answered(mails, me)?;
    let mut to = addr::split(&one.to);
    to.extend(addr::split(&last.reply.to));
    let to = addr::without(&to, &[me]);
    let mut cc = to.clone();
    cc.extend(addr::split(&last.reply.cc));
    let cc = addr::without(&cc, &[me]).split_off(to.len());
    Some(Answer {
        to: to.join(", "),
        cc: cc.join(", "),
        ..one
    })
}

/// The message a reply answers: the last someone else sent, else the last.
fn answered<'a>(mails: &'a [Mail], me: &str) -> Option<&'a Mail> {
    let me = me.to_ascii_lowercase();
    let theirs = |m: &&Mail| me.is_empty() || !m.from.to_ascii_lowercase().contains(&me);
    mails.iter().rev().find(theirs).or(mails.last())
}

/// The reply as Gmail takes it: an RFC 2822 message, base64url. Header
/// values lose any line break (a body cannot smuggle a header in), and a
/// subject outside ASCII is RFC 2047-encoded.
pub(crate) fn raw(a: &Answer, body: &str) -> String {
    let m = super::mime::message(&head(a), &super::mime::Node::text("plain", body));
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(m)
}

/// To, Cc, Subject and the threading headers, each one line.
pub(crate) fn head(a: &Answer) -> Vec<String> {
    let mut h = vec![format!("To: {}", addr::header(&a.to))];
    if !a.cc.trim().is_empty() {
        h.push(format!("Cc: {}", addr::header(&a.cc)));
    }
    h.push(format!(
        "Subject: {}",
        encoded(&a.subject.replace(['\r', '\n'], " "))
    ));
    if !a.in_reply_to.is_empty() {
        h.push(format!("In-Reply-To: {}", a.in_reply_to));
        h.push(format!("References: {}", a.references));
    }
    h
}

fn encoded(s: &str) -> String {
    match s.is_ascii() {
        true => s.to_string(),
        false => {
            let b = base64::engine::general_purpose::STANDARD.encode(s);
            format!("=?UTF-8?B?{b}?=")
        }
    }
}

impl Gmail {
    /// Send `raw` (RFC 2822, base64url) in the thread: `gmail.modify` covers it.
    pub(crate) fn send(&mut self, thread_id: &str, raw: &str) -> Result<(), String> {
        let body = serde_json::json!({"raw": raw, "threadId": thread_id});
        self.post("/messages/send", body).map(|_| ())
    }
}

#[cfg(test)]
#[path = "draft_tests.rs"]
mod tests;
