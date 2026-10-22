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
    /// `u` pressed once: a second unsubscribes (`unsub`).
    pub unsub_armed: bool,
    /// `q`: the quoted earlier replies unfolded.
    pub quotes: bool,
    /// Pictures from the web drawn: as the thread opens, until `i`.
    pub web: bool,
    /// `w`: asked for a window of its own, until the app opens one.
    pub pop_out: bool,
    /// How big each landed picture is (`readfit`).
    pub sizes: super::readfit::Sizes,
    /// Older mails opened by a click, `x` for all, and a count of the
    /// changes for the layout's key (`readfold`).
    pub open: std::collections::BTreeSet<usize>,
    pub all: bool,
    pub folds: u32,
    pub(crate) laid: std::sync::Mutex<Option<(reader::Key, std::sync::Arc<reader::Laid>)>>,
    /// The pane's size when last drawn, for what moves the scroll between
    /// keys; and a reply just sent: its thread opens again at its end.
    pub seen: std::cell::Cell<(u16, u16)>,
    pub to_end: bool,
}

impl Reading {
    pub(crate) fn new(thread: Thread) -> Self {
        Self {
            thread,
            mails: None,
            scroll: 0,
            plain: false,
            quotes: false,
            unsub_armed: false,
            web: false,
            pop_out: false,
            sizes: Default::default(),
            open: Default::default(),
            all: false,
            folds: 0,
            laid: std::sync::Mutex::new(None),
            seen: Default::default(),
            to_end: false,
        }
    }

    /// `v` (`plain`) or `q`: flip it when the thread has something to flip, and
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
            false => "no quoted replies in this thread",
        }
    }
}
