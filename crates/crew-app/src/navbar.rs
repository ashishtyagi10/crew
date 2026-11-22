//! The activity bar: the nav's left edge, one icon per section and the settings gear at its
//! foot — VS Code's left edge, in crew's own card.
//!
//! The bar is the same five columns the collapsed rail always was, in both states, so an icon
//! keeps its column when the panel beside it opens or closes. (Collapsed, the card is dropped
//! level with the panes' first row (`panellevel`), so the icons sit up to a row lower there.) Open, the bar is only the icons; the
//! open section is marked by an accent stripe down its left edge. Collapsed, the pane numbers the
//! rail always carried follow the icons, so going to a pane is still one click from anywhere.
//!
//! **Where everything is** is answered once, by [`row_of`] and [`at_row`]: the draw and the
//! click read the same mapping, so an icon can never be drawn on one row and answer on another.
use crew_render::CellView;

use crate::navsection::NavSection;

/// The bar's columns inside the card — the rail's five.
pub(crate) const BAR_COLS: u16 = crate::navrail::RAIL_COLS - 2;
/// The column a badge stands in: the bar's right edge.
const BADGE_COL: u16 = BAR_COLS - 1;
/// Rows between one icon and the next — the icon and a row of air.
const PITCH: u16 = 2;
/// Rows of air above the first icon and below the gear: a drawn icon is taller than its row
/// (`navicons`) and would be cut by the card's edge without it.
const MARGIN: u16 = 1;
/// Rows the icons take from the top of the bar, the air above and under them included.
/// Collapsed, the pane numbers start a rule's row below this.
pub(crate) const HEAD_ROWS: u16 = MARGIN + PITCH * NavSection::ALL.len() as u16;

/// Something on the bar a click can mean.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BarHit {
    Section(NavSection),
    Settings,
}

/// The content row `hit` is drawn on, in a bar `rows` tall. `None` when the bar is too short to
/// hold it — the gear goes first, then the icons from the bottom up.
pub(crate) fn row_of(hit: BarHit, rows: u16) -> Option<u16> {
    let row = match hit {
        BarHit::Section(s) => {
            let i = NavSection::ALL.iter().position(|&n| n == s)? as u16;
            MARGIN + i * PITCH
        }
        BarHit::Settings => rows.checked_sub(1 + MARGIN).filter(|&r| r >= HEAD_ROWS)?,
    };
    (row < rows).then_some(row)
}

/// What a click on content row `row` of a bar `rows` tall means: an icon's whole slot — its row
/// and the row of air under it, which the drawn icon reaches into — not just its row, as VS
/// Code's whole square is the target.
pub(crate) fn at_row(row: u16, rows: u16) -> Option<BarHit> {
    let all = NavSection::ALL.into_iter().map(BarHit::Section);
    all.chain([BarHit::Settings])
        .find(|&h| row_of(h, rows).is_some_and(|r| row == r || row == r + 1))
}

/// The marks the bar carries for what it cannot show: which pane wants you (the rail's own
/// mark and colour) on the Panes icon, and how many things wait on you on the Activity one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Badges {
    pub panes: Option<(char, (u8, u8, u8))>,
    pub waiting: usize,
}

/// The loudest mark among `panes`, in the rail's own priority: attention, then work in progress,
/// then lines not yet read.
pub(crate) fn panes_badge(
    panes: &[crate::panelist::PaneRow],
    spin: char,
) -> Option<(char, (u8, u8, u8))> {
    let t = crew_theme::theme();
    let rank =
        |p: &crate::panelist::PaneRow| match (p.attention, p.busy, p.activity || p.unread > 0) {
            (Some((glyph, on)), _, _) => on.then_some((3, glyph, t.bell)),
            (None, true, _) => Some((2, spin, crate::accentink::accent_ink())),
            (None, false, true) => Some((1, '\u{25cf}', t.activity)),
            _ => None,
        };
    let best = panes.iter().filter_map(rank).max_by_key(|(r, _, _)| *r)?;
    Some((best.1, best.2))
}

/// The bar's cells, in its own coordinates: the stripe on the open section (`None`: collapsed,
/// nothing marked open) and the badges. The icons are paint ([`crate::navicons`]).
pub(crate) fn bar_cells(open: Option<NavSection>, badges: Badges, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::accentink::accent_ink();
    let mut out = Vec::new();
    let mut put = |col: u16, row: u16, c: char, fg| {
        out.push(CellView {
            col,
            row,
            c,
            fg,
            bg: t.page_bg,
            ..Default::default()
        })
    };
    for s in NavSection::ALL {
        let Some(row) = row_of(BarHit::Section(s), rows) else {
            continue;
        };
        // The icon itself is drawn (`navicons`); the cells carry what is TEXT about it — the
        // stripe that marks the open section, and the badge.
        if open == Some(s) {
            put(0, row, '\u{258e}', accent);
        }
        let badge = match s {
            NavSection::Panes => badges.panes,
            NavSection::Activity if badges.waiting > 0 => {
                let n = std::char::from_digit(badges.waiting.min(9) as u32, 10).unwrap_or('+');
                Some((if badges.waiting > 9 { '+' } else { n }, t.bell))
            }
            _ => None,
        };
        if let Some((c, fg)) = badge {
            put(BADGE_COL, row, c, fg);
        }
    }
    out
}

/// The bar hover last drawn, encoded (0: none) — so a move from one icon to the next repaints,
/// as the nav's rows do (`panehover`).
static HOVER: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Publish this move's bar hover. `true` when it changed — a frame is owed to it.
pub(crate) fn publish_hover(h: Option<BarHit>) -> bool {
    let code = match h {
        None => 0,
        Some(BarHit::Settings) => 1,
        Some(BarHit::Section(s)) => {
            2 + NavSection::ALL.iter().position(|&n| n == s).unwrap_or(0) as u8
        }
    };
    HOVER.swap(code, std::sync::atomic::Ordering::Relaxed) != code
}

/// The rule between the bar and the panel, one column down the card's interior.
pub(crate) fn divider_cells(col: u16, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    (0..rows)
        .map(|row| CellView {
            col,
            row,
            c: '\u{2502}',
            fg: t.border_normal,
            bg: t.page_bg,
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
#[path = "navbar_tests.rs"]
mod tests;
