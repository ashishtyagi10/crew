//! The nav's Inbox and Far lists from the keyboard, as the Explorer's tree is (`explorerkeys`).
//!
//! Both lists answered the pointer only: every unread thread, every recent folder, was a reach
//! for the mouse in an app driven from the keys. `/nav inbox` and `/nav far` now hand that list
//! the keyboard: ↑/↓ (or k/j) move a cursor bar, Home/End go to the ends, Enter opens the item
//! exactly as a click on it does, Esc gives the keys back.
//!
//! **Whose keys.** As with the tree: the list holds them only while the pane focused when it
//! took them is still focused, the bar is not, and the nav still shows that list. Any other
//! focus change — a click, Cmd+I, a new pane — ends the hold by itself (`nav_keys_settle`).
//! The cursor scrolls the list through the one window the draw, the click and the wheel read
//! (`navlistscroll`), so it never walks off the panel.
use crew_render::CellView;
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::navsection::NavSection;

/// The sections whose list takes the keys this way.
pub(crate) const LISTS: [NavSection; 2] = [NavSection::Inbox, NavSection::Far];

/// A list's hold on the keyboard: whose list, its cursor item, and the pane focused when it
/// took them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ListKeys {
    pub section: NavSection,
    pub item: usize,
    owner: usize,
}

impl CrewApp {
    /// Whether the Inbox or Far list has the keyboard now (see the module doc).
    pub(crate) fn list_has_keys(&self) -> bool {
        let held = self.sidebar.lists.keys;
        held.is_some_and(|k| k.owner == self.focused && self.nav_section() == k.section)
            && !self.input.focused
            && self.config.show_nav
            && !self.config.nav_collapsed
    }

    /// Whether any of the nav's lists — the tree, the Inbox, Far — has the keyboard.
    pub(crate) fn nav_has_keys(&self) -> bool {
        self.tree_has_keys() || self.list_has_keys()
    }

    /// Once a frame: a hold that was lost is gone for good, so coming back to the same pane
    /// does not hand the list the keys again.
    pub(crate) fn nav_keys_settle(&mut self) {
        if self.sidebar.explorer.keys.is_some() && !self.tree_has_keys() {
            self.sidebar.explorer.keys = None;
        }
        if self.sidebar.lists.keys.is_some() && !self.list_has_keys() {
            self.sidebar.lists.keys = None;
        }
    }

    /// `/nav inbox` or `/nav far`: hand that list the keyboard, the cursor on the first item
    /// the panel shows, so a scrolled list stays where the wheel left it.
    pub(crate) fn list_keys_take(&mut self, s: NavSection) {
        self.config.show_nav = true;
        self.input.focused = false;
        self.sidebar.explorer.keys = None;
        let first = self.list_rows().map_or(0, |r| self.list_window(s, r).0);
        self.sidebar.lists.keys = Some(ListKeys {
            section: s,
            item: first,
            owner: self.focused,
        });
    }

    /// A key while one of the nav's lists holds the keyboard. `true` when it was the list's.
    pub(crate) fn nav_key(&mut self, event: &KeyEvent) -> bool {
        self.explorer_key(event) || self.list_key(&event.logical_key, event.state.is_pressed())
    }

    /// A key for the Inbox or Far list, on the logical key — the part a test can drive. Any key
    /// it has no use for ends the hold and goes on to the pane.
    pub(crate) fn list_key(&mut self, key: &Key, pressed: bool) -> bool {
        if !pressed || !self.list_has_keys() {
            return false;
        }
        let Some(mut keys) = self.sidebar.lists.keys else {
            return false;
        };
        let len = crate::navlistscroll::listed(keys.section);
        let last = len.saturating_sub(1);
        let item = keys.item.min(last);
        let ch = |c: &str| matches!(key, Key::Character(s) if s.as_str() == c);
        match key {
            Key::Named(NamedKey::ArrowDown) => keys.item = (item + 1).min(last),
            _ if ch("j") => keys.item = (item + 1).min(last),
            Key::Named(NamedKey::ArrowUp) => keys.item = item.saturating_sub(1),
            _ if ch("k") => keys.item = item.saturating_sub(1),
            Key::Named(NamedKey::Home) => keys.item = 0,
            Key::Named(NamedKey::End) => keys.item = last,
            Key::Named(NamedKey::Enter) => {
                // The pane it opens takes the keys, as after a click.
                self.sidebar.lists.keys = None;
                let k = (len > 0).then_some(item);
                match keys.section {
                    NavSection::Inbox => self.inbox_open_item(k),
                    _ => self.far_open_item(k),
                }
                self.redraw();
                return true;
            }
            Key::Named(NamedKey::Escape) => {
                self.sidebar.lists.keys = None;
                self.redraw();
                return true;
            }
            // A modifier on its own is the start of a chord, not a key for anything.
            Key::Named(NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Super) => {
                return true
            }
            _ => {
                self.sidebar.lists.keys = None;
                return false;
            }
        }
        self.sidebar.lists.keys = Some(keys);
        if let Some(rows) = self.list_rows() {
            self.sidebar.lists.follow(keys.section, keys.item, rows);
        }
        self.redraw();
        true
    }

    /// The cursor bar over section `s`'s cells, when its list holds the keys and the cursor is
    /// in `(first, shown)`, the window drawn: across every row its item takes.
    pub(crate) fn list_bar(
        &self,
        out: &mut Vec<CellView>,
        s: NavSection,
        (first, shown): (usize, usize),
        cols: u16,
    ) {
        let Some(k) = self.sidebar.lists.keys.filter(|k| k.section == s) else {
            return;
        };
        // Where Enter acts: the item clamped to a list that may have shrunk since.
        let item = k
            .item
            .min(crate::navlistscroll::listed(s).saturating_sub(1));
        if !self.list_has_keys() || item < first || item >= first + shown {
            return;
        }
        let (top, per) = match s {
            NavSection::Inbox => (crate::navinbox::TOP, crate::navinbox::PER),
            _ => (crate::navfar::TOP, 1),
        };
        let row = top + (item - first) as u16 * per;
        for r in row..row + per {
            crate::explorerdraw::bar(out, r, cols);
        }
    }

    /// How many content rows the open section's panel has.
    fn list_rows(&self) -> Option<u16> {
        let (sb, _, _) = self.nav_hit_geometry()?;
        let (cw, ch, ..) = self.frame_geometry()?;
        Some(crate::layout::card_inner_cells(sb.w, sb.h, cw, ch).1)
    }
}

#[cfg(test)]
#[path = "navlistkeys_tests.rs"]
mod tests;
