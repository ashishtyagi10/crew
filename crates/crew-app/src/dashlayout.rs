//! Where the `/dash` pane's bands sit and how wide its ring is: the row
//! divisions, the minimum width each band needs, and the cell writer they
//! share.
//!
//! Split from [`crate::dashpane`] for the line cap, along the line between
//! working out the layout and painting into it.
use crew_render::CellView;

/// The dashboard's bands, top to bottom. The three above the history are
/// fixed — they draw a machine, and a machine's readings do not get truer with
/// more rows. A band is drawn only when the pane has the rows for it, and the
/// order is the priority: a short pane keeps the machine and loses the
/// history.
pub(crate) const SYS_TOP: u16 = 1;

pub(crate) const SYS_ROWS: u16 = crate::sysdials::DASH.rows;

pub(crate) const NET_TOP: u16 = SYS_TOP + SYS_ROWS + 1;

pub(crate) const NET_ROWS: u16 = 3;

pub(crate) const USE_TOP: u16 = NET_TOP + NET_ROWS + 1;

/// The two below it are the histories, and they DO get truer with more rows —
/// so they take the pane's slack, in this order. See [`layout`].
pub(crate) const HEAT_ROW_MAX: u16 = 3;

pub(crate) const COST_MIN: u16 = 3;

pub(crate) const COST_MAX: u16 = 12;

/// How the dashboard divides `rows` below the machine, for one frame — the one
/// derivation the text and the drawing both read.
///
/// The bands were four `const`s, which is one layout for every pane: on a full
/// window they finished 55% of the way down and left 340 pixels of paper under
/// them, with a week of hours drawn as a one-row strip and seven days of cost
/// as a three-row smear. Same division `usagepane` makes, for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Rows one day of the heatmap claims.
    pub(crate) heat_h: u16,
    /// Row the cost curve starts on; its legend is on the row above.
    pub(crate) cost_top: u16,
    /// Chart rows the cost curve gets — `0` when the pane cannot hold it.
    pub(crate) cost_rows: u16,
}

pub(crate) fn layout(rows: u16) -> Layout {
    const DAYS: u16 = crate::usageledger::DAYS as u16;
    // Both histories at their floor, plus the cost band's legend row and a row
    // of air under the pane's last chart.
    let floor = USE_TOP + DAYS + 2 + COST_MIN + 1;
    let slack = rows.saturating_sub(floor);
    // The heatmap has first claim: it is the one chart here with a week of
    // readings in it, and at one row a day it is a strip you squint at.
    let heat_h = (1 + slack / DAYS).min(HEAT_ROW_MAX);
    let slack = slack.saturating_sub((heat_h - 1) * DAYS);
    let cost_top = USE_TOP + DAYS * heat_h + 2;
    // …and the cost curve takes what is left, between its floor and a cap:
    // past that, seven readings are a blob with a stroke on it.
    let cost_rows = match rows.checked_sub(cost_top + 1) {
        Some(room) if room >= COST_MIN => (COST_MIN + slack).min(COST_MAX).min(room),
        _ => 0,
    };
    Layout {
        heat_h,
        cost_top,
        cost_rows,
    }
}

/// Columns the CPU curve keeps beside the dials, at minimum. A curve narrower
/// than this is a shape you cannot read a trend off.
pub(crate) const CURVE_MIN: u16 = 18;

/// Columns the dial block claims on the left of the SYSTEM band; the CPU
/// curve fills what is left.
///
/// The dials give width back rather than squeezing the curve out: in a
/// narrow dash they draw smaller and keep their block within what is left.
pub(crate) fn ring_w(cols: u16) -> u16 {
    crate::sysdials::DASH_COLS
        .min(cols.saturating_sub(CURVE_MIN))
        .max(crate::sysdials::MIN_COLS)
}

/// Minimum pane the dashboard will draw in at all.
pub(crate) const MIN_COLS: u16 = 46;

pub(crate) fn put(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    cols: u16,
) {
    for (i, ch) in s.chars().enumerate() {
        let col = col + i as u16;
        if col >= cols {
            break;
        }
        out.push(CellView {
            col,
            row,
            c: ch,
            fg,
            bg: crew_theme::theme().page_bg,
            ..Default::default()
        });
    }
}
