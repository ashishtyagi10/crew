//! The Explorer from the keyboard — VS Code's Cmd+Shift+E.
//!
//! The tree was mouse-only: every folder opened, every file previewed, was a reach for the
//! pointer in an app driven from the keys. Cmd+Shift+E now opens the nav on the Explorer and
//! hands the tree the keyboard: ↑/↓ (or k/j) move a cursor row, → or Enter opens a folder or
//! previews a file in the tree's one viewer, ← closes a folder or steps to its parent,
//! Home/End go to the ends, a letter jumps to the next name starting with it
//! ([`crate::explorerfind`]), Esc — or the chord again — gives the keys back.
//!
//! **Whose keys.** The tree holds them only while nothing else took them: the pane that had
//! focus when the tree took over (or the viewer it opened since) is still focused and the bar is
//! not. Any other focus change — a click, Cmd+I, a new pane — ends the mode by itself, so no
//! path that moves focus has to know the tree exists. While it holds them the focused pane draws
//! unfocused (`render.rs`), so the ring says where typing goes.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::navsection::NavSection;

/// The tree's hold on the keyboard: its cursor row, and the pane focused when it took them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TreeKeys {
    pub row: usize,
    owner: usize,
}

impl CrewApp {
    /// Whether the tree has the keyboard now (see the module doc).
    pub(crate) fn tree_has_keys(&self) -> bool {
        let held = self.sidebar.explorer.keys;
        held.is_some_and(|k| k.owner == self.focused)
            && !self.input.focused
            && self.config.show_nav
            && !self.config.nav_collapsed
            && self.nav_section() == NavSection::Explorer
    }

    /// Cmd+Shift+E: give the tree the keyboard, opening the nav on the Explorer; or, when it
    /// has them, give them back.
    pub(crate) fn explorer_keys_toggle(&mut self) {
        if self.tree_has_keys() {
            self.sidebar.explorer.keys = None;
            return;
        }
        self.config.show_nav = true;
        if self.config.nav_collapsed || self.nav_section() != NavSection::Explorer {
            self.pick_nav_section(NavSection::Explorer);
        }
        self.input.focused = false;
        // The cursor starts on the focused viewer's file: reveal it now, not a frame later.
        self.explorer_reveal();
        let items = self.sidebar.explorer.items(&self.explorer_root());
        let picked = self.sidebar.explorer.picked.as_deref();
        let row = items.iter().position(|i| Some(i.path.as_path()) == picked);
        self.sidebar.explorer.keys = Some(TreeKeys {
            row: row.unwrap_or(0),
            owner: self.focused,
        });
        self.explorer_follow();
    }

    /// A key while the tree holds the keyboard. `true` when it was the tree's; any key it has
    /// no use for ends the mode and goes on to the pane.
    pub(crate) fn explorer_key(&mut self, event: &KeyEvent) -> bool {
        self.explorer_tree_key(&event.logical_key, event.state.is_pressed())
    }

    /// [`Self::explorer_key`] on the logical key — the part a test can drive.
    pub(crate) fn explorer_tree_key(&mut self, key: &Key, pressed: bool) -> bool {
        if !pressed || !self.tree_has_keys() {
            return false;
        }
        let items = self.sidebar.explorer.items(&self.explorer_root());
        let Some(mut keys) = self.sidebar.explorer.keys else {
            return false;
        };
        let last = items.len().saturating_sub(1);
        let row = keys.row.min(last);
        let item = items.get(row).cloned();
        let ch = |c: &str| matches!(key, Key::Character(s) if s.as_str() == c);
        let opens = ch(" ") || matches!(key, Key::Named(NamedKey::ArrowRight | NamedKey::Enter));
        match key {
            _ if self.explorer_find_key(key, &items, &mut keys.row) => {}
            Key::Named(NamedKey::ArrowDown) => keys.row = (row + 1).min(last),
            _ if ch("j") => keys.row = (row + 1).min(last),
            Key::Named(NamedKey::ArrowUp) => keys.row = row.saturating_sub(1),
            _ if ch("k") => keys.row = row.saturating_sub(1),
            Key::Named(NamedKey::Home) => keys.row = 0,
            Key::Named(NamedKey::End) => keys.row = last,
            _ if opens => {
                match item {
                    Some(i) if i.more.is_some() => {}
                    // An open folder: → steps in, as VS Code's does.
                    Some(i) if i.dir && i.open => keys.row = (row + 1).min(last),
                    Some(i) if i.dir => self.sidebar.explorer.toggle(&i),
                    Some(i) => {
                        self.sidebar.explorer.picked = Some(i.path.clone());
                        self.explorer_open(i.path);
                        // Browsing goes on: the viewer just focused is now the owner.
                        keys.owner = self.focused;
                    }
                    None => {}
                }
            }
            Key::Named(NamedKey::ArrowLeft) => match item {
                Some(i) if i.dir && i.open && i.more.is_none() => self.sidebar.explorer.toggle(&i),
                Some(i) => {
                    let parent = items[..row].iter().rposition(|p| p.depth + 1 == i.depth);
                    keys.row = parent.unwrap_or(row);
                }
                None => {}
            },
            Key::Named(NamedKey::Escape) => {
                self.sidebar.explorer.keys = None;
                self.redraw();
                return true;
            }
            // A modifier on its own is the start of a chord, not a key for anything.
            Key::Named(NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Super) => {
                return true
            }
            _ => {
                self.sidebar.explorer.keys = None;
                return false;
            }
        }
        self.sidebar.explorer.keys = Some(keys);
        self.explorer_follow();
        self.redraw();
        true
    }

    /// Scroll the tree so its cursor row is on screen.
    fn explorer_follow(&mut self) {
        if let Some(keys) = self.sidebar.explorer.keys {
            self.explorer_show_row(keys.row, None);
        }
    }

    /// Scroll the tree as little as it takes to put item `row` on screen — or, given the list's
    /// `len`, with an off-screen row brought to the middle, so a jump lands with its
    /// neighbours around it rather than on the panel's edge.
    pub(crate) fn explorer_show_row(&mut self, row: usize, centre_in: Option<usize>) {
        let Some((sb, _, _)) = self.nav_hit_geometry() else {
            return;
        };
        let Some((cw, ch, ..)) = self.frame_geometry() else {
            return;
        };
        let (_, rows) = crate::layout::card_inner_cells(sb.w, sb.h, cw, ch);
        let shown = usize::from(rows.saturating_sub(crate::explorerdraw::TOP)).max(1);
        let e = &mut self.sidebar.explorer;
        let off = !(e.scroll..e.scroll + shown).contains(&row);
        match centre_in {
            Some(len) if off => {
                e.scroll = row.saturating_sub(shown / 2).min(len.saturating_sub(shown))
            }
            _ => e.scroll = e.scroll.clamp(row.saturating_sub(shown - 1), row),
        }
    }
}

#[cfg(test)]
#[path = "explorerkeys_tests.rs"]
mod tests;
