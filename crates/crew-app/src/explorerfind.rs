//! Type-to-find in the Explorer tree while it holds the keys — Finder's and VS Code's.
//!
//! A tree walked one row at a time is slow to cross: `src` twenty rows down was twenty presses
//! of ↓. Now a letter jumps: typing starts a prefix, and the cursor goes to the first listed
//! item from where it stands whose name starts with it (any case, wrapping round). The same
//! single letter again steps to the next such row, so `s s s` walks every `s…`. j and k stay
//! motions until a prefix is being typed — then they are letters too, so `jest` finds `jest`.
//! A space likewise opens, unless it extends a prefix.
//!
//! The prefix shows in the rule (`EXPLORER find: src`, muted when nothing matches, the cursor
//! left where it was) and goes after [`PAUSE_MS`] without a key, on Esc (a second Esc then
//! gives the keys back), on Backspace to empty, or on any movement key. Only the rows the tree
//! lists are searched, never the disk. Ageing out needs no timer: [`Find::live`] reads the age
//! wherever the prefix is used, and the poll loop clears a stale one so its legend is repainted
//! away once ([`CrewApp::explorer_find_expire`]).
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::explorer::Item;

/// How long a prefix lives without a key.
pub(crate) const PAUSE_MS: u64 = 1000;

/// The prefix being typed: lower-cased, when its last key came, and whether it matched.
#[derive(Clone, Debug, Default)]
pub(crate) struct Find {
    prefix: String,
    at: u64,
    found: bool,
}

impl Find {
    /// The prefix and whether it matched — `None` when there is none or it has aged out.
    pub(crate) fn live(&self, now: u64) -> Option<(&str, bool)> {
        let fresh = now.saturating_sub(self.at) < PAUSE_MS;
        (!self.prefix.is_empty() && fresh).then_some((self.prefix.as_str(), self.found))
    }
}

/// The clock the prefix ages by: crew's own, or the one a test set.
pub(crate) fn now() -> u64 {
    #[cfg(test)]
    if let Some(t) = TEST_NOW.with(std::cell::Cell::get) {
        return t;
    }
    crate::anim::now_ms()
}

#[cfg(test)]
thread_local! {
    pub(crate) static TEST_NOW: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

impl CrewApp {
    /// A key for the find, ahead of every other the tree takes. `true` when it was the find's
    /// (the cursor `row` moved to a match, or the prefix changed). A key it passes on clears the
    /// prefix — a movement key ends a find — except a modifier on its own: Shift starts a capital.
    pub(crate) fn explorer_find_key(&mut self, key: &Key, items: &[Item], row: &mut usize) -> bool {
        let now = now();
        let m = self.mods.state();
        // Cmd/Ctrl/Alt + a letter is a chord for someone else, never a letter typed.
        let chord = m.control_key() || m.alt_key() || m.super_key();
        let find = &mut self.sidebar.explorer.find;
        if find.live(now).is_none() {
            find.prefix.clear();
        }
        let typing = !find.prefix.is_empty();
        let from = (*row).min(items.len().saturating_sub(1));
        let start = match key {
            Key::Named(NamedKey::Shift | NamedKey::Control | NamedKey::Alt | NamedKey::Super) => {
                return false
            }
            Key::Named(NamedKey::Escape) if typing => {
                find.prefix.clear();
                return true;
            }
            Key::Named(NamedKey::Backspace) if typing => {
                find.prefix.pop();
                from
            }
            Key::Character(s) if !chord && typed(s, typing) => {
                let c = s.to_lowercase();
                // The same single letter again: the next row that starts with it.
                if find.prefix == c {
                    from + 1
                } else {
                    find.prefix.push_str(&c);
                    from
                }
            }
            _ => {
                find.prefix.clear();
                return false;
            }
        };
        find.at = now;
        let hit = seek(items, &find.prefix, start);
        find.found = hit.is_some();
        *row = hit.unwrap_or(*row);
        true
    }

    /// Clear a prefix that aged out, or that outlived the tree's hold on the keys. `true` when
    /// one was cleared: its legend is owed one repaint.
    pub(crate) fn explorer_find_expire(&mut self, now: u64) -> bool {
        let held = self.tree_has_keys();
        let find = &mut self.sidebar.explorer.find;
        let stale = !find.prefix.is_empty() && (find.live(now).is_none() || !held);
        if stale {
            find.prefix.clear();
        }
        stale
    }
}

/// Whether `s` is typed into the find: anything printable — but j, k and space only while a
/// prefix is already being typed, as until then they move and open.
fn typed(s: &str, typing: bool) -> bool {
    let printable = !s.is_empty() && s.chars().all(|c| !c.is_control());
    printable && (typing || !matches!(s, "j" | "k" | " "))
}

/// The first listed item from `from` on, wrapping round, whose name starts with `prefix`.
fn seek(items: &[Item], prefix: &str, from: usize) -> Option<usize> {
    let n = items.len();
    (0..n).map(|k| (from + k) % n.max(1)).find(|&k| {
        let i = &items[k];
        i.more.is_none() && i.name.to_lowercase().starts_with(prefix)
    })
}

/// The rule's legend for a live `prefix` on a panel `cols` wide: `find: src`, its start given
/// up to `…` before its end — the end is what was just typed.
pub(crate) fn legend(prefix: &str, cols: u16) -> String {
    let room = crate::boxdraw::title_budget(cols).saturating_sub("EXPLORER find: ".len());
    // By display width, as the header budgets: a CJK prefix counted in chars overflowed it.
    if crate::chatwidth::str_w(prefix) <= room {
        return format!("find: {prefix}");
    }
    let mut tail: Vec<char> = Vec::new();
    let mut w = 1; // the ellipsis
    for c in prefix.chars().rev() {
        w += crate::chatwidth::char_w(c);
        if w > room.max(1) {
            break;
        }
        tail.push(c);
    }
    let tail: String = tail.into_iter().rev().collect();
    format!("find: \u{2026}{tail}")
}

#[cfg(test)]
#[path = "explorerfind_tests.rs"]
mod tests;
