//! A cut nav row, shown whole when the pointer rests on it.
//!
//! The nav is narrow and its list rows are long: a Far folder's place read `/var/folde…`, a
//! sender's subject stopped at its fourth word, a deep file's name lost its extension — and
//! nothing on screen ever said the rest. Now, when the pointer has rested [`DELAY_MS`] on a
//! row the nav had to cut (`navcut` says which — the frame's own draw, not a second guess), a
//! small card stands just right of the nav at the row's height with the row's whole text: the
//! folder's full path, the sender and the whole subject, the file's name, the pane's title.
//! A row drawn whole gets no card. Moving off the row, a key, a click or the wheel puts it
//! away; it never takes a click (it stands over the panes, not over the row).
//!
//! Like the caret's blink (`cursorblink`) it costs no frames while it waits: each frame notes
//! when the card falls due and the poll asks for that one repaint ([`NavTip::take_due`]).
use std::cell::RefCell;

use crate::app::CrewApp;
use crate::navsection::NavSection;

/// How long the pointer rests on a cut row before its card appears — long enough that a
/// pointer passing down the list raises nothing.
pub(crate) const DELAY_MS: u64 = 400;

/// The row a card would be for: the section, the item, the texts its row draws (what
/// `navcut` may report cut), and the whole text the card says, one paragraph each.
pub(crate) struct Target {
    pub key: (NavSection, usize),
    pub drawn: Vec<String>,
    pub whole: Vec<String>,
}

/// The card's clock: the row under the pointer, when it got there, and the one repaint owed.
#[derive(Default)]
pub(crate) struct NavTip {
    key: Option<(NavSection, usize)>,
    /// When the pointer came to the row (the `anim` clock).
    pub(crate) since: u64,
    /// When the card appears.
    pub(crate) show_at: u64,
    due: Option<u64>,
    shown: bool,
    /// The texts the last panel draw cut (`navcut::record`, in `push_sections`).
    pub(crate) cut: RefCell<Vec<String>>,
}

impl NavTip {
    /// Note this frame's row (`key`), whether it was cut, and the last key, click or scroll;
    /// say whether the card is up. A new row restarts the wait; any input since the pointer
    /// came puts the card away until it moves to another row.
    pub(crate) fn observe(
        &mut self,
        key: Option<(NavSection, usize)>,
        cut: bool,
        last_input: u64,
        now: u64,
    ) -> bool {
        if key != self.key {
            self.key = key;
            self.since = now;
            self.show_at = now + DELAY_MS;
        }
        let live = key.is_some() && cut && last_input <= self.since;
        self.due = (live && now < self.show_at).then_some(self.show_at);
        self.shown = live && now >= self.show_at;
        self.shown
    }

    /// Whether a repaint is owed: the card has come due, or one is up that a key since put
    /// away (a key that changed nothing else would leave it standing). Cleared here, so a
    /// frame that never lands cannot turn it into a stream.
    pub(crate) fn take_due(&mut self, now: u64, last_input: u64) -> bool {
        let due = self.due.is_some_and(|at| now >= at);
        if due {
            self.due = None;
        }
        let gone = self.shown && last_input > self.since;
        if gone {
            self.shown = false;
        }
        due || gone
    }
}

impl CrewApp {
    /// The row under the pointer that a card could be for, in the open section.
    pub(crate) fn nav_tip_target(&self) -> Option<Target> {
        if !self.cursor_in || self.help_open || self.config.nav_collapsed {
            return None;
        }
        let s = self.nav_section();
        let (k, drawn, whole) = match s {
            NavSection::Far => {
                let k = self.far_item_at_cursor()?;
                let dir = crate::farrecent::list().get(k)?.clone();
                let (name, at) = crate::navfar::name_and_parent(&dir);
                (k, vec![name, at], vec![crate::disktile::short_path(&dir)])
            }
            NavSection::Inbox => {
                let k = self.inbox_item_at_cursor()?;
                let th = crate::inboxglance::unread().get(k)?.clone();
                let who = crate::inbox::wire::sender_name(&th.from);
                let what = crate::navinbox::subject(&th).to_string();
                (k, vec![who.clone(), what.clone()], vec![who, what])
            }
            NavSection::Explorer => {
                let k = self.explorer_item_at_cursor()?;
                let items = self.sidebar.explorer.items(&self.explorer_root());
                let item = items.get(k).filter(|i| i.more.is_none())?;
                let name = crate::explorerdraw::shown_name(item);
                (k, vec![name], vec![item.name.clone()])
            }
            NavSection::Panes => {
                let i = self.pane_at_panes_list()?;
                let title = self.panes.get(i)?.title_text();
                (i, vec![title.clone()], vec![title])
            }
            _ => return None,
        };
        Some(Target {
            key: (s, k),
            drawn,
            whole,
        })
    }

    /// This frame's card: the clock stepped, and — once the pointer has rested on a cut row —
    /// the card pushed onto `scenes`.
    pub(crate) fn push_nav_tip(&mut self, scenes: &mut Vec<crew_render::PaneScene>, now: u64) {
        let target = self.nav_tip_target();
        let cut = target.as_ref().is_some_and(|t| {
            let cut = self.nav_tip.cut.borrow();
            t.drawn.iter().any(|d| cut.contains(d))
        });
        let key = target.as_ref().map(|t| t.key);
        if !self.nav_tip.observe(key, cut, self.last_input_ms, now) {
            return;
        }
        if let Some(t) = target {
            scenes.extend(self.nav_tip_scene(t.key.0, &t.whole));
        }
    }

    /// Whether the card owes a repaint now — the poll's one question (`take_due`).
    pub(crate) fn nav_tip_due(&mut self, now: u64) -> bool {
        self.nav_tip.take_due(now, self.last_input_ms)
    }
}

#[cfg(test)]
#[path = "navtip_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "navtipshot_tests.rs"]
mod shot_tests;
