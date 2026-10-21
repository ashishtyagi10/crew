//! An opened thread and how it is being read.
use super::api::Mail;
use super::reader;
use super::rich::Rich;
use super::wire::Thread;

/// An opened thread: its row, its messages once they arrive, the scroll,
/// the two reading toggles, and its layout (`reader::Reading::laid`).
pub(crate) struct Reading {
    pub thread: Thread,
    pub mails: Option<Vec<Mail>>,
    pub scroll: usize,
    /// `v`: each message's other version (the plain part of an HTML mail).
    pub plain: bool,
    /// `z`: quoted history unfolded.
    pub quotes: bool,
    /// `i`: pictures from the web drawn.
    pub web: bool,
    pub(crate) laid: std::sync::Mutex<Option<(reader::Key, std::sync::Arc<reader::Laid>)>>,
}

impl Reading {
    pub(crate) fn new(thread: Thread) -> Self {
        Self {
            thread,
            mails: None,
            scroll: 0,
            plain: false,
            quotes: false,
            web: false,
            laid: std::sync::Mutex::new(None),
        }
    }

    /// `v` (`plain`) or `z`: flip it when the thread has something to flip, and
    /// the line the foot says.
    pub(crate) fn toggle(&mut self, plain: bool) -> &'static str {
        let mails = self.mails.as_deref().unwrap_or_default();
        let has = |f: fn(&Rich) -> bool| mails.iter().any(|m| f(&m.rich));
        match plain {
            true if has(|x| x.alt.is_some()) => {
                self.plain = !self.plain;
                if self.plain {
                    "the plain-text version \u{b7} v for the formatted one"
                } else {
                    ""
                }
            }
            true => "this mail has only one version",
            false
                if has(|x| {
                    x.quote.is_some() || x.alt.as_ref().is_some_and(|a| a.quote.is_some())
                }) =>
            {
                self.quotes = !self.quotes;
                ""
            }
            false => "no quoted history in this thread",
        }
    }
}
