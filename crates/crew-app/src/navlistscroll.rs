//! How far the nav's Inbox and Far lists are scrolled. Each list used to stop at the panel's
//! bottom, the rest out of reach; the wheel over the panel now moves a window down it, as
//! it does over the Explorer. The window is worked out in ONE place ([`window`], through
//! [`ListScroll::window`]) and both the draw and the click read it, so a click always lands
//! on the row drawn under it.
use crate::app::CrewApp;
use crate::navsection::NavSection;

/// Items scrolled off the top of each list, as the last wheel left them — a wheel line is one
/// item, so a notch over the Inbox moves three threads (six rows).
#[derive(Debug, Default)]
pub(crate) struct ListScroll {
    inbox: usize,
    far: usize,
    /// The keyboard's hold on a list, when one has it (`navlistkeys`).
    pub(crate) keys: Option<crate::navlistkeys::ListKeys>,
}

impl ListScroll {
    fn slot(&mut self, s: NavSection) -> Option<&mut usize> {
        match s {
            NavSection::Inbox => Some(&mut self.inbox),
            NavSection::Far => Some(&mut self.far),
            _ => None,
        }
    }

    /// The window of section `s`'s list a panel `rows` tall shows: the first item, and how
    /// many. What the panel draws and what a click on it reads.
    pub(crate) fn window(&self, s: NavSection, rows: u16) -> (usize, usize) {
        let off = match s {
            NavSection::Inbox => self.inbox,
            NavSection::Far => self.far,
            _ => 0,
        };
        let (len, fits) = extent(s, rows);
        window(off, len, fits)
    }

    /// Scroll section `s`'s list so item `k` is in the window a panel `rows` tall shows: the
    /// keyboard's cursor never walks off the panel. Through the same [`window`] the draw reads.
    pub(crate) fn follow(&mut self, s: NavSection, k: usize, rows: u16) {
        let (len, fits) = extent(s, rows);
        let Some(off) = self.slot(s) else {
            return;
        };
        let cur = window(*off, len, fits).0;
        let wanted = cur.max((k + 1).saturating_sub(fits.max(1))).min(k);
        *off = window(wanted, len, fits).0;
    }
}

/// How many items section `s` lists, scrolled or not.
pub(crate) fn listed(s: NavSection) -> usize {
    extent(s, 0).0
}

/// The items a list of `len` shows from scroll `off` with room for `fits`: the first one and
/// how many. The first stops where the last item reaches the bottom — a list that shrank
/// since it was scrolled shows from as far down as it now goes.
pub(crate) fn window(off: usize, len: usize, fits: usize) -> (usize, usize) {
    let first = off.min(len.saturating_sub(fits));
    (first, (len - first).min(fits))
}

/// How many items section `s` lists, and how many a panel `rows` tall has room for.
fn extent(s: NavSection, rows: u16) -> (usize, usize) {
    match s {
        NavSection::Inbox => (
            crate::navinbox::listed(&crate::inboxglance::state(), &crate::inboxglance::unread()),
            crate::navinbox::fits(rows),
        ),
        NavSection::Far => (
            crate::farrecent::list().len(),
            usize::from(rows.saturating_sub(crate::navfar::TOP)),
        ),
        _ => (0, 0),
    }
}

impl CrewApp {
    /// The window of section `s`'s list a panel `rows` tall shows ([`ListScroll::window`]).
    pub(crate) fn list_window(&self, s: NavSection, rows: u16) -> (usize, usize) {
        self.sidebar.lists.window(s, rows)
    }

    /// A wheel over the Inbox or Far panel scrolls its list (`lines` < 0 moves down it).
    /// `true` when the wheel was the panel's.
    pub(crate) fn list_scroll(&mut self, lines: i32) -> bool {
        let s = self.nav_section();
        let Some((_, rows)) = self.section_row(s) else {
            return false;
        };
        let (len, fits) = extent(s, rows);
        let Some(off) = self.sidebar.lists.slot(s) else {
            return false;
        };
        // From where the panel is DRAWN, not the stored offset: a list that shrank since the
        // last wheel would otherwise spend the first notch walking back to the clamp. Stored
        // through the same window the draw reads.
        let cur = window(*off, len, fits).0;
        let wanted = (cur as i64 - i64::from(lines)).max(0) as usize;
        *off = window(wanted, len, fits).0;
        true
    }
}

#[cfg(test)]
#[path = "navlistscroll_tests.rs"]
mod tests;
