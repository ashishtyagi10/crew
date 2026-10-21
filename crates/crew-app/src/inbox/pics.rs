//! The open thread's pictures (goal 2026-10-20-mail-reader, phase 2): what
//! each source has become — still coming, decoded, or nothing to draw — held
//! in memory for the thread and dropped with it. Nothing is written to disk.
//!
//! A picture the mail carries (`cid:`, and image attachments) is fetched as
//! the thread opens: its bytes are already Google's, so asking for them tells
//! no one anything. A picture from the web is fetched only on `i`, because
//! fetching one tells its sender the mail was opened, when and from where.
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use super::worker::Ask;
use super::InboxPane;
use crate::viewpane::bitmap::Bitmap;

/// At most this many web pictures are fetched per message.
const WEB_PER_MAIL: usize = 20;
/// Image attachments drawn under their message.
pub(crate) const ATTACHED: usize = 4;
/// An attachment bigger than this is named, not fetched to be drawn.
const ATTACHED_BYTES: u64 = 10 << 20;

pub(crate) enum Pic {
    Loading,
    Ready(Arc<Bitmap>),
    /// Unreadable, too big, a tracking pixel, or the fetch failed.
    Failed,
}

pub(crate) struct Pics {
    map: HashMap<String, Pic>,
    pub(crate) tx: Sender<(String, Option<Bitmap>)>,
    rx: Receiver<(String, Option<Bitmap>)>,
}

impl Default for Pics {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            map: HashMap::new(),
            tx,
            rx,
        }
    }
}

/// What a picture is stored under: a mail's own pictures by message (two
/// mails can both have a `cid:logo`), a web picture by its address.
pub(crate) fn key(msg: &str, src: &str) -> String {
    let src = src.split("#rows=").next().unwrap_or(src);
    match src.starts_with("cid:") || src.starts_with("att:") {
        true => format!("{msg}|{src}"),
        false => src.to_string(),
    }
}

impl Pics {
    pub(crate) fn get(&self, key: &str) -> Option<&Pic> {
        self.map.get(key)
    }

    pub(crate) fn set(&mut self, key: String, bm: Option<Bitmap>) {
        let pic = bm.map_or(Pic::Failed, |b| Pic::Ready(Arc::new(b)));
        self.map.insert(key, pic);
    }

    /// Mark `key` as asked for; false when it already was.
    fn claim(&mut self, key: &str) -> bool {
        !self.map.contains_key(key) && self.map.insert(key.to_string(), Pic::Loading).is_none()
    }

    /// What the web fetcher sent back; true when anything did.
    pub(crate) fn drain(&mut self) -> bool {
        let got: Vec<_> = self.rx.try_iter().collect();
        let any = !got.is_empty();
        for (k, bm) in got {
            self.set(k, bm);
        }
        any
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }
}

impl InboxPane {
    /// The thread just arrived: ask for every picture it carries.
    pub(crate) fn fetch_carried(&mut self) {
        let Some(mails) = self.reading.as_ref().and_then(|r| r.mails.clone()) else {
            return;
        };
        for m in &mails {
            let named = |p: &super::body::Part| p.cid.as_ref().map(|c| format!("cid:{c}"));
            let drawn: Vec<&String> = m.rich.pics.iter().chain(web_pics(&m.rich)).collect();
            for part in &m.parts {
                let src = match named(part).filter(|s| drawn.contains(&s)) {
                    Some(src) => src,
                    None if attached(part) => format!("att:{}", part.att.as_deref().unwrap_or("")),
                    None => continue,
                };
                if self.pics.claim(&key(&m.id, &src)) {
                    self.ask(Ask::Picture {
                        key: key(&m.id, &src),
                        msg: m.id.clone(),
                        att: part.att.clone(),
                        data: part.data.clone(),
                    });
                }
            }
        }
    }

    /// `i`: the web's pictures for the open thread. The line to say.
    pub(crate) fn show_web(&mut self) -> String {
        let Some(r) = self.reading.as_mut() else {
            return String::new();
        };
        let mails = r.mails.as_deref().unwrap_or_default();
        let mut want = Vec::new();
        for m in mails.iter() {
            let urls = web_pics(&m.rich).filter(|s| !s.starts_with("cid:"));
            want.extend(urls.take(WEB_PER_MAIL).map(|u| (key(&m.id, u), u.clone())));
        }
        if want.is_empty() {
            return "no pictures from the web in this thread".into();
        }
        r.web = true;
        want.retain(|(k, _)| self.pics.claim(k));
        let n = want.len();
        super::picfetch::web(want, self.pics.tx.clone());
        format!(
            "fetching {n} picture{} from the web",
            if n == 1 { "" } else { "s" }
        )
    }
}

/// The pictures a message would draw with the web's included.
pub(crate) fn web_pics(r: &super::rich::Rich) -> impl Iterator<Item = &String> {
    r.with_web.iter().flat_map(|w| w.pics.iter())
}

/// An image attachment small enough to draw.
pub(crate) fn attached(p: &super::body::Part) -> bool {
    p.cid.is_none() && p.att.is_some() && p.mime.starts_with("image/") && p.size <= ATTACHED_BYTES
}

#[cfg(test)]
#[path = "pics_tests.rs"]
mod tests;
