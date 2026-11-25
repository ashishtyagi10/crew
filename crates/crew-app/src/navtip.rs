//! A cut nav row, shown whole when the pointer rests on it.
//!
//! The nav is narrow and its list rows are long: a Far folder's place read `/var/folde…`, a
//! sender's subject stopped at its fourth word, a deep file's name lost its extension — and
//! nothing on screen ever said the rest. Now, when the pointer has rested [`DELAY_MS`] on a
//! row the nav had to cut (`navcut` says which — the frame's own draw, not a second guess), a
//! small card stands just right of the nav at the row's height with the row's whole text: the
//! folder's full path, the sender and the whole subject, the file's name, the pane's title.
//! A row drawn whole gets no card. Moving off the row, a key, a click or the wheel puts it
//! away; it never takes a click (it stands over the panes, not over the row). A row that came
//! under a resting pointer some other way — the wheel scrolled it there, a key or a new list
//! moved it — waits for the pointer to move before its wait begins: the card answers a
//! pointer brought to a row, not a list that slid under one.
//!
//! Like the caret's blink (`cursorblink`) it costs no frames while it waits: each frame notes
//! when the card falls due and the poll asks for that one repaint ([`NavTip::take_due`]).
use std::cell::RefCell;

use crate::app::CrewApp;
use crate::navsection::NavSection;

/// How long the pointer rests on a cut row before its card appears — long enough that a
/// pointer passing down the list raises nothing.
pub(crate) const DELAY_MS: u64 = 400;

/// The row a card would be for: the section, the item, the panel rows it is drawn on and the
/// texts they draw (what `navcut` may report cut there), and the whole text the card says,
/// one paragraph each.
pub(crate) struct Target {
    pub key: (NavSection, usize),
    pub rows: Vec<u16>,
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
    /// Where the pointer was at the last look, and whether the row came under it while it
    /// rested (no wait runs until it moves).
    at: Option<(f32, f32)>,
    parked: bool,
    /// The texts the last panel draw cut, by row (`navcut::record`, in `push_sections`).
    pub(crate) cut: RefCell<Vec<crate::navcut::Cut>>,
}

impl NavTip {
    /// Note this frame's row (`key`), whether it was cut, the last key, click or scroll and
    /// the pointer (`at`); say whether the card is up. A new row the pointer moved to restarts
    /// the wait; one that came under a still pointer parks it until the pointer moves. Any
    /// input since the pointer came puts the card away until it moves to another row.
    pub(crate) fn observe(
        &mut self,
        key: Option<(NavSection, usize)>,
        cut: bool,
        (last_input, now): (u64, u64),
        at: (f32, f32),
    ) -> bool {
        let moved = self.at != Some(at);
        self.at = Some(at);
        if key != self.key || (self.parked && moved) {
            self.key = key;
            self.restart(now, !moved);
        }
        let live = key.is_some() && cut && !self.parked && last_input <= self.since;
        self.due = (live && now < self.show_at).then_some(self.show_at);
        self.shown = live && now >= self.show_at;
        self.shown
    }

    /// Whether a repaint is owed: the card has come due, or one is up that a key since put
    /// away (a key that changed nothing else would leave it standing). Cleared here, so a
    /// frame that never lands cannot turn it into a stream.
    /// A parked row whose pointer has since moved starts its wait here: a move within one row
    /// may cost no frame, and the wait must not hang on one.
    pub(crate) fn take_due(&mut self, now: u64, last_input: u64, at: (f32, f32)) -> bool {
        if self.parked && self.at != Some(at) {
            self.at = Some(at);
            self.restart(now, false);
            self.due = Some(self.show_at);
        }
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

    /// The wait begins again at `now` — or, `parked`, not until the pointer moves.
    fn restart(&mut self, now: u64, parked: bool) {
        self.since = now;
        self.show_at = now + DELAY_MS;
        self.parked = parked;
    }
}

impl CrewApp {
    /// The row under the pointer that a card could be for, in the open section.
    pub(crate) fn nav_tip_target(&self) -> Option<Target> {
        if !self.cursor_in || self.help_open || self.config.nav_collapsed {
            return None;
        }
        let s = self.nav_section();
        let (k, rows, drawn, whole) = match s {
            NavSection::Far => {
                let k = self.far_item_at_cursor()?;
                let row = self.section_row(s)?.0;
                let dir = crate::farrecent::list().get(k)?.clone();
                let (name, at) = crate::navfar::name_and_parent(&dir);
                let whole = vec![crate::disktile::short_path(&dir)];
                (k, vec![row], vec![name, at], whole)
            }
            NavSection::Inbox => {
                let k = self.inbox_item_at_cursor()?;
                // A thread is two rows, the sender's and the subject's.
                let (row, per) = (self.section_row(s)?.0, crate::navinbox::PER);
                let top = row - (row - crate::navinbox::TOP) % per;
                let th = crate::inboxglance::unread().get(k)?.clone();
                let who = crate::inbox::wire::sender_name(&th.from);
                let what = crate::navinbox::subject(&th).to_string();
                (
                    k,
                    vec![top, top + 1],
                    vec![who.clone(), what.clone()],
                    vec![who, what],
                )
            }
            NavSection::Explorer => {
                let k = self.explorer_item_at_cursor()?;
                let row = self.section_row(s)?.0;
                let items = self.explorer_items();
                let item = items.get(k).filter(|i| i.more.is_none())?;
                let name = crate::explorerdraw::shown_name(item);
                (k, vec![row], vec![name], vec![item.name.clone()])
            }
            NavSection::Panes => {
                let i = self.pane_at_panes_list()?;
                let (rows, drawn, whole) = self.pane_tip(i)?;
                (i, rows, drawn, whole)
            }
            _ => return None,
        };
        Some(Target {
            key: (s, k),
            rows,
            drawn,
            whole,
        })
    }

    /// This frame's card: the clock stepped, and — once the pointer has rested on a cut row —
    /// the card pushed onto `scenes`.
    pub(crate) fn push_nav_tip(&mut self, scenes: &mut Vec<crew_render::PaneScene>, now: u64) {
        let target = self.nav_tip_target();
        // Cut on the item's own rows: a sibling's cut parent is the same text on another row.
        let cut = target.as_ref().is_some_and(|t| {
            let cut = self.nav_tip.cut.borrow();
            cut.iter()
                .any(|(r, s)| t.rows.contains(r) && t.drawn.contains(s))
        });
        let key = target.as_ref().map(|t| t.key);
        let input = (self.last_input_ms, now);
        if !self.nav_tip.observe(key, cut, input, self.cursor) {
            return;
        }
        if let Some(t) = target {
            scenes.extend(self.nav_tip_scene(t.key.0, &t.whole));
        }
    }

    /// Whether the card owes a repaint now — the poll's one question (`take_due`).
    pub(crate) fn nav_tip_due(&mut self, now: u64) -> bool {
        self.nav_tip.take_due(now, self.last_input_ms, self.cursor)
    }
}

#[cfg(test)]
#[path = "navtip_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "navtipshot_tests.rs"]
mod shot_tests;

#[cfg(test)]
#[path = "navtiprow_tests.rs"]
mod row_tests;
