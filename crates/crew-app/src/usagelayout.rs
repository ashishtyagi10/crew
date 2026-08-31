//! Where everything sits in the `/usage` pane, and how its numbers read: the
//! band heights, the ring's centre and radii, the label column, and the
//! compact forms for money and days.
//!
//! Split from [`crate::usagepane`] for the line cap, along the line between
//! working out the layout and painting into it.
use crate::usageledger::DAYS;

/// Row the heatmap's first day sits on. Everything below it moves with the
/// height the pane actually has — see [`layout`].
pub(crate) const HEAT_TOP: u16 = 2;

/// The TOKENS band: its legend row, then [`RING_ROWS`] the donut is drawn on.
/// The ring used to share the legend's row with it — at radius 3.6 canvas
/// units it is nearly two rows tall in each direction, and its top arc landed
/// on the word TOKENS.
pub(crate) const RING_ROWS: u16 = 5;

pub(crate) const SPLIT_ROWS: u16 = 1 + RING_ROWS;

/// Chart rows the cost band is worth drawing on at all, and the most it will
/// take: seven readings spread over more than this stop being a curve with a
/// shape and become a slow blob with a stroke on top.
pub(crate) const COST_MIN: u16 = 2;

pub(crate) const COST_MAX: u16 = 14;

/// Rows one day of the heatmap may claim. At one row a week of hours is a
/// strip you squint at; the rows a tall pane can spare buy the grid a band per
/// day you can actually compare across.
pub(crate) const HEAT_ROW_MAX: u16 = 3;

/// The donut's centre column, and the row of the band the label in its hole
/// sits on — ONE derivation, read by both the drawing and the text.
///
/// They used to be two: the ring was painted on a canvas shifted one column
/// right of the column the hole's label was centred on, so the total's first
/// character sat on the ring's left arc instead of inside the hole. A hole is
/// exactly wide enough for the number it was sized for; one column of drift
/// is the whole margin.
pub(crate) const RING_CX: f32 = 7.0;

/// Radii of the ring, in canvas units (one unit = one cell width).
pub(crate) const RING_R_OUT: f32 = 3.6;

pub(crate) const RING_R_IN: f32 = 2.2;

/// Row, within the band, the ring is centred on — the middle of [`RING_ROWS`],
/// which is a row's centre because the count is odd, so the hole's label lands
/// on a whole row rather than straddling two.
pub(crate) const RING_ROW: u16 = 1 + RING_ROWS / 2;

/// Columns of labels down the left of the heatmap (`Mon `), and the air kept
/// to the right of every chart.
pub(crate) const LABEL_W: u16 = 4;

pub(crate) const RIGHT_PAD: u16 = 2;

/// How the pane divides `rows` between its three bands, for one frame — the
/// one derivation the text and the drawing both read, so a label can never
/// land on a row the chart beside it was drawn from a different sum.
///
/// The bands used to be four `const`s. A pane is not one height: at a quarter
/// tile the cost band asked for five rows, could not have them, and was
/// dropped whole while six rows sat empty under the donut; at a full window
/// the three of them finished 45% of the way down and the rest of the pane was
/// paper. Both ends are the same bug — a layout that is a stack of fixed
/// sizes and not a division of what there is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    /// Rows one day of the heatmap claims.
    pub(crate) heat_h: u16,
    /// Row the TOKENS legend sits on.
    pub(crate) split_top: u16,
    /// Row the COST PER DAY legend sits on.
    pub(crate) cost_top: u16,
    /// Chart rows the cost band gets — `0` when the pane cannot hold it.
    pub(crate) cost_rows: u16,
}

pub(crate) fn layout(rows: u16) -> Layout {
    // Every band at its floor: the heatmap and its hour ticks, a gap, the
    // TOKENS band, a gap, and the cost band's legend + floor + axis row.
    let floor = HEAT_TOP + DAYS as u16 + 1 + 1 + SPLIT_ROWS + 1 + 1 + COST_MIN + 1;
    let slack = rows.saturating_sub(floor);
    // The heatmap has first claim on the slack: it is the pane's headline, and
    // the rows it buys go straight into the one chart here with a week of
    // readings in it.
    let heat_h = (1 + slack / DAYS as u16).min(HEAT_ROW_MAX);
    let slack = slack.saturating_sub((heat_h - 1) * DAYS as u16);
    let split_top = HEAT_TOP + DAYS as u16 * heat_h + 2;
    let cost_top = split_top + SPLIT_ROWS + 1;
    // …and the cost curve takes what is left of it, between its own floor and
    // cap, never past what the pane has under the legend.
    let cost_rows = match rows.checked_sub(cost_top + 2) {
        Some(room) if room >= COST_MIN => (COST_MIN + slack).min(COST_MAX).min(room),
        _ => 0,
    };
    Layout {
        heat_h,
        split_top,
        cost_top,
        cost_rows,
    }
}

/// Tokens in the fewest characters that still say the magnitude: `184k`,
/// `2.3M`. The footer's `fmt_tokens` renders 2.25M as `2250.0k`, which is
/// seven characters and does not fit in a donut's hole.
pub fn compact(n: u64) -> String {
    // Every tier is capped at four significant characters plus a suffix, so
    // no reading can outgrow the hole it is written in — including the
    // implausible ones, which is the point: a widget's layout must not depend
    // on the data staying reasonable.
    const K: u64 = 1_000;
    match n {
        0..=999 => n.to_string(),
        _ if n < K * K => format!("{}k", n / K),
        _ if n < K * K * K => format!("{:.1}M", n as f64 / (K * K) as f64),
        _ if n < K * K * K * K => format!("{:.1}G", n as f64 / (K * K * K) as f64),
        _ => format!("{:.1}T", n as f64 / (K * K * K * K) as f64),
    }
}

/// Micro-USD as `$1.23` (or `$0.004` while it is still small — a session that
/// cost less than a cent is the common case and rounding it to `$0.00` says
/// crew is free).
pub fn money(microusd: u64) -> String {
    let usd = microusd as f64 / 1_000_000.0;
    if usd >= 0.01 || usd == 0.0 {
        format!("${usd:.2}")
    } else {
        format!("${usd:.3}")
    }
}

/// Day labels, oldest first, ending in `today`.
pub(crate) fn day_labels() -> Vec<String> {
    // Weekday names would need a calendar; "6d" … "now" needs none, and says
    // the thing the row actually is: how long ago.
    (0..DAYS)
        .map(|i| match DAYS - 1 - i {
            0 => "now".to_string(),
            n => format!("{n}d"),
        })
        .collect()
}
