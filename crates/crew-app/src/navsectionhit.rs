//! Where the pointer is in a section's panel — the row every list section (Explorer, Inbox,
//! Far) reads its hover and its clicks from — and the one hover mark the list sections share,
//! so a move from one row to the next repaints, as the nav's other rows do (`panehover`).
use crate::app::CrewApp;
use crate::navsection::NavSection;

impl CrewApp {
    /// The panel's content row under the cursor, and the panel's rows, when the open section
    /// is `s` and the nav is open.
    pub(crate) fn section_row(&self, s: NavSection) -> Option<(u16, u16)> {
        if self.config.nav_collapsed || self.nav_section() != s {
            return None;
        }
        let (sb, ch, _) = self.nav_hit_geometry()?;
        if !crate::chrome::point_in(sb, self.cursor.0, self.cursor.1) {
            return None;
        }
        // Measured from the card's outer top edge: its border is row 0 of that.
        let rel = ((self.cursor.1 - sb.y) / ch).floor() as i32 - 1;
        let (_, rows) = self
            .frame_geometry()
            .map(|(cw, ch, ..)| crate::layout::card_inner_cells(sb.w, sb.h, cw, ch))?;
        let row = u16::try_from(rel).ok()?;
        (row < rows).then_some((row, rows))
    }

    /// The list row under the pointer in the Inbox or Far section — what is lit there.
    pub(crate) fn list_hover(&self) -> Option<(NavSection, usize)> {
        let s = self.nav_section();
        let k = match s {
            NavSection::Inbox => self.inbox_item_at_cursor(),
            NavSection::Far => self.far_item_at_cursor(),
            _ => None,
        }?;
        Some((s, k))
    }

    /// Whether the pointer is over the Inbox or Far panel's rows — every row there answers a
    /// click, so every row is a control.
    pub(crate) fn on_list_panel(&self) -> bool {
        [NavSection::Inbox, NavSection::Far]
            .into_iter()
            .any(|s| self.section_row(s).is_some_and(|(r, _)| r > 0))
    }
}

/// The list hover last drawn, encoded (0: none).
static HOVER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Publish this move's list hover. `true` when it changed.
pub(crate) fn publish_hover(h: Option<(NavSection, usize)>) -> bool {
    let code = h.map_or(0, |(s, k)| {
        let i = NavSection::ALL.iter().position(|&n| n == s).unwrap_or(0);
        1 + i + NavSection::ALL.len() * k
    });
    HOVER.swap(code, std::sync::atomic::Ordering::Relaxed) != code
}
