//! `/usage` — what crew has spent, drawn.
//!
//! Every number this pane shows was already in `usage.jsonl` and reachable
//! only as two countdowns in the chat footer. Seven days of spend has a
//! *shape* — the hours you work, the days you did not, the session that cost
//! four times its neighbours — and none of it survives being printed as a
//! total.
//!
//! Three views, stacked: a heatmap of tokens by hour over the week, a donut
//! splitting the tokens sent from the tokens received, and an area chart of
//! what each day cost.
use crew_render::{CellView, Paint};

use crate::boxdraw::section_header;
use crate::palette::accent;
use crate::plot::pie::{self, Slice};
use crate::plot::{area, heatmap, Canvas};
use crate::usageledger::{Buckets, DAYS, HOURS};

pub struct UsagePane {
    /// The buckets last drawn, refreshed on a clock so the pane keeps up with
    /// requests landing while it is open without re-reading the ledger every
    /// frame.
    buckets: Buckets,
    last_ms: u64,
}

/// How often the pane re-buckets the ledger.
const REFRESH_MS: u64 = 2_000;

impl UsagePane {
    pub fn new() -> Self {
        let now = crate::anim::now_ms();
        Self {
            buckets: crate::usageledger::buckets(wall_ms()),
            last_ms: now,
        }
    }

    /// Returns true when something changed and the pane should repaint.
    pub fn refresh(&mut self) -> bool {
        let now = crate::anim::now_ms();
        if now.saturating_sub(self.last_ms) < REFRESH_MS {
            return false;
        }
        self.last_ms = now;
        let next = crate::usageledger::buckets(wall_ms());
        let changed = next != self.buckets;
        self.buckets = next;
        changed
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        cells(&self.buckets, cols, rows)
    }

    pub fn paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        paint(&self.buckets, cols, rows, aspect)
    }
}

impl Default for UsagePane {
    fn default() -> Self {
        Self::new()
    }
}

fn wall_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Row the heatmap's first day sits on. Everything below it moves with the
/// height the pane actually has — see [`layout`].
const HEAT_TOP: u16 = 2;
/// The TOKENS band: its legend row, then [`RING_ROWS`] the donut is drawn on.
/// The ring used to share the legend's row with it — at radius 3.6 canvas
/// units it is nearly two rows tall in each direction, and its top arc landed
/// on the word TOKENS.
const RING_ROWS: u16 = 5;
const SPLIT_ROWS: u16 = 1 + RING_ROWS;
/// Chart rows the cost band is worth drawing on at all, and the most it will
/// take: seven readings spread over more than this stop being a curve with a
/// shape and become a slow blob with a stroke on top.
const COST_MIN: u16 = 2;
const COST_MAX: u16 = 14;
/// Rows one day of the heatmap may claim. At one row a week of hours is a
/// strip you squint at; the rows a tall pane can spare buy the grid a band per
/// day you can actually compare across.
const HEAT_ROW_MAX: u16 = 3;
/// The donut's centre column, and the row of the band the label in its hole
/// sits on — ONE derivation, read by both the drawing and the text.
///
/// They used to be two: the ring was painted on a canvas shifted one column
/// right of the column the hole's label was centred on, so the total's first
/// character sat on the ring's left arc instead of inside the hole. A hole is
/// exactly wide enough for the number it was sized for; one column of drift
/// is the whole margin.
const RING_CX: f32 = 7.0;
/// Radii of the ring, in canvas units (one unit = one cell width).
const RING_R_OUT: f32 = 3.6;
const RING_R_IN: f32 = 2.2;
/// Row, within the band, the ring is centred on — the middle of [`RING_ROWS`],
/// which is a row's centre because the count is odd, so the hole's label lands
/// on a whole row rather than straddling two.
const RING_ROW: u16 = 1 + RING_ROWS / 2;
/// Columns of labels down the left of the heatmap (`Mon `), and the air kept
/// to the right of every chart.
const LABEL_W: u16 = 4;
const RIGHT_PAD: u16 = 2;

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
struct Layout {
    /// Rows one day of the heatmap claims.
    heat_h: u16,
    /// Row the TOKENS legend sits on.
    split_top: u16,
    /// Row the COST PER DAY legend sits on.
    cost_top: u16,
    /// Chart rows the cost band gets — `0` when the pane cannot hold it.
    cost_rows: u16,
}

fn layout(rows: u16) -> Layout {
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
fn day_labels() -> Vec<String> {
    // Weekday names would need a calendar; "6d" … "now" needs none, and says
    // the thing the row actually is: how long ago.
    (0..DAYS)
        .map(|i| match DAYS - 1 - i {
            0 => "now".to_string(),
            n => format!("{n}d"),
        })
        .collect()
}

pub fn cells(b: &Buckets, cols: u16, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if cols < 24 || rows < 6 {
        return out;
    }
    let put = |out: &mut Vec<CellView>, s: &str, col: u16, row: u16, fg: (u8, u8, u8)| {
        for (i, ch) in s.chars().enumerate() {
            let col = col + i as u16;
            if col + 1 >= cols {
                break;
            }
            out.push(CellView {
                col,
                row,
                c: ch,
                fg,
                bg: t.page_bg,
                ..Default::default()
            });
        }
    };

    // Header: the week's totals, which is what the charts below are of.
    out.extend(section_header(
        "USAGE",
        cols,
        t.border_normal,
        accent(),
        t.page_bg,
    ));
    put(
        &mut out,
        &format!(
            "{} \u{00b7} {} in \u{00b7} {} out \u{00b7} 7 days",
            money(b.cost_microusd),
            compact(b.tok_in),
            compact(b.tok_out),
        ),
        1,
        1,
        t.ink,
    );

    // Heatmap: a band per day, an hour per column. Each day's label is
    // centred on the band it names, so a two- or three-row day does not read
    // as a label with an unlabelled stripe under it.
    let l = layout(rows);
    let heat_end = HEAT_TOP + DAYS as u16 * l.heat_h;
    if rows > heat_end {
        for (i, label) in day_labels().into_iter().enumerate() {
            let row = HEAT_TOP + i as u16 * l.heat_h + (l.heat_h - 1) / 2;
            put(&mut out, &label, 1, row, t.text_muted);
        }
        // Hour ticks under the grid, at midnight / 06 / 12 / 18.
        let grid_w = cols.saturating_sub(LABEL_W + RIGHT_PAD);
        let hour_col = |h: usize| LABEL_W + (h as u16 * grid_w) / HOURS as u16;
        for h in [0usize, 6, 12, 18] {
            put(
                &mut out,
                &format!("{h:02}"),
                hour_col(h),
                heat_end,
                t.text_muted,
            );
        }
    }

    // The in/out donut's legend and the total in its hole.
    if rows > l.split_top + SPLIT_ROWS {
        put(&mut out, "TOKENS", 1, l.split_top, t.text_muted);
        let total = b.tok_in + b.tok_out;
        let hole = compact(total);
        let start = (RING_CX - hole.chars().count() as f32 / 2.0)
            .round()
            .max(0.0) as u16;
        put(&mut out, &hole, start, l.split_top + RING_ROW, t.ink);
        let pct = |v: u64| match total {
            0 => 0,
            _ => (v * 100 / total).min(100),
        };
        // The two readings flank the ring's own centre row, so each sits
        // opposite the arc it is naming. Each is written in its slice's
        // colour: the words ARE the key, which is why the ring no longer
        // carries a pair of swatch dots beside it.
        put(
            &mut out,
            &format!("in   {}  {}%", compact(b.tok_in), pct(b.tok_in)),
            13,
            l.split_top + RING_ROW - 1,
            accent(),
        );
        put(
            &mut out,
            &format!("out  {}  {}%", compact(b.tok_out), pct(b.tok_out)),
            13,
            l.split_top + RING_ROW + 1,
            t.ansi[13],
        );
    }

    // The daily-cost chart's label and the day it peaked.
    if l.cost_rows > 0 {
        let axis = l.cost_top + 1 + l.cost_rows;
        put(&mut out, "COST PER DAY", 1, l.cost_top, t.text_muted);
        let peak = b.daily_cost.iter().copied().max().unwrap_or(0);
        put(
            &mut out,
            &format!("peak {}", money(peak)),
            cols.saturating_sub(14),
            l.cost_top,
            t.text_muted,
        );
        put(&mut out, "6d ago", 1, axis, t.text_muted);
        put(
            &mut out,
            "today",
            cols.saturating_sub(RIGHT_PAD + 5),
            axis,
            t.text_muted,
        );
    }
    out
}

pub fn paint(b: &Buckets, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if cols < 24 || rows < 6 {
        return out;
    }
    let grid_w = cols.saturating_sub(LABEL_W + RIGHT_PAD);
    let l = layout(rows);

    // Heatmap. Cold cells keep a faint trace so the grid reads as a grid; hot
    // ones walk the theme's own gradient, the same ramp the meters use. The
    // grid is always DAYS x HOURS cells; what the pane's height buys is how
    // many rows each of those cells is drawn over.
    if rows > HEAT_TOP + DAYS as u16 * l.heat_h {
        let mut c = Canvas::new(grid_w, DAYS as u16 * l.heat_h, aspect);
        let (w, h) = c.size();
        heatmap::draw(
            &mut c,
            (0.0, 0.0, w, h),
            &b.hourly,
            DAYS,
            HOURS,
            0.12,
            &|k: f32| {
                let color = crate::modernring::pole_mix(k).unwrap_or_else(accent);
                (color, 0.10 + 0.90 * k.powf(0.6))
            },
        );
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(f32::from(LABEL_W), f32::from(HEAT_TOP))),
        );
    }

    // In / out donut, on the band's own rows under the TOKENS legend. Its
    // centre column is `RING_CX` measured in the PANE, so the canvas is
    // shifted to put it there rather than each end guessing.
    if rows > l.split_top + SPLIT_ROWS {
        const SHIFT: f32 = 1.0;
        let mut c = Canvas::new(14, RING_ROWS, aspect);
        let (_, h) = c.size();
        let centre = (RING_CX - SHIFT, h / 2.0);
        let slices = [
            Slice::new(b.tok_in as f32, accent()),
            Slice::new(b.tok_out as f32, t.ansi[13]),
        ];
        pie::donut(
            &mut c,
            centre,
            RING_R_OUT,
            RING_R_IN,
            &slices,
            t.border_normal,
        );
        // Punch the hole back to the page, so the total written in it is read
        // off the page rather than off the ring's inner edge.
        pie::dot(&mut c, centre, RING_R_IN, t.page_bg, 1.0);
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(SHIFT, f32::from(l.split_top + 1))),
        );
    }

    // Cost per day, over whatever rows the division left it.
    if l.cost_rows > 0 {
        let peak = b.daily_cost.iter().copied().max().unwrap_or(0).max(1);
        let samples: Vec<f32> = b
            .daily_cost
            .iter()
            .map(|&v| (v as f32 / peak as f32).clamp(0.0, 1.0))
            .collect();
        let w_cells = cols.saturating_sub(2 + RIGHT_PAD);
        let mut c = Canvas::new(w_cells, l.cost_rows, aspect);
        let (w, h) = c.size();
        area::draw(&mut c, (0.0, 0.0, w, h), &samples, t.ansi[11]);
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(1.0, f32::from(l.cost_top + 1))),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        cells, compact, layout, money, paint, COST_MAX, COST_MIN, HEAT_ROW_MAX, HEAT_TOP, LABEL_W,
        RING_ROW, RING_R_IN, SPLIT_ROWS,
    };
    use crate::usageledger::{Buckets, DAYS, HOURS};

    /// A week with exactly one busy hour, in a named day/hour.
    fn one_hot(day: usize, hour: usize) -> Buckets {
        let mut hourly = vec![0u64; DAYS * HOURS];
        hourly[day * HOURS + hour] = 10_000;
        Buckets {
            hourly,
            daily_cost: vec![0; DAYS],
            tok_in: 7_000,
            tok_out: 3_000,
            cost_microusd: 1_500_000,
        }
    }

    #[test]
    fn the_heatmaps_hot_cell_lands_on_its_own_day_and_hour() {
        let _g = crate::app::theme_test_guard();
        // At every band height the pane's division can hand the grid: the
        // cell's row is what changes, the day it belongs to is not.
        for rows in [22u16, 30, 40] {
            let h = layout(rows).heat_h;
            for (day, hour) in [(0usize, 0usize), (3, 12), (DAYS - 1, HOURS - 1)] {
                let p = paint(&one_hot(day, hour), 60, rows, 2.0);
                let end = f32::from(HEAT_TOP + DAYS as u16 * h);
                let hot = p
                    .iter()
                    .filter(|r| r.y >= f32::from(HEAT_TOP) && r.y < end)
                    .max_by(|a, b| a.alpha.total_cmp(&b.alpha))
                    .expect("the heatmap drew");
                let band = ((hot.y - f32::from(HEAT_TOP)) / f32::from(h)).floor() as usize;
                assert_eq!(band, day, "{rows} rows: day {day} is in its own band");
                // Its column: the grid spans LABEL_W..cols-RIGHT_PAD.
                let grid_w = 60.0 - f32::from(LABEL_W) - 2.0;
                let col = ((hot.x - f32::from(LABEL_W)) / grid_w * HOURS as f32).floor() as usize;
                assert_eq!(col, hour, "hour {hour} sits in its own column");
            }
        }
    }

    #[test]
    fn every_day_of_the_week_gets_a_row() {
        let _g = crate::app::theme_test_guard();
        let mut hourly = vec![0u64; DAYS * HOURS];
        for d in 0..DAYS {
            hourly[d * HOURS + 5] = 1_000 * (d as u64 + 1);
        }
        let b = Buckets {
            hourly,
            ..one_hot(0, 0)
        };
        let h = f32::from(layout(40).heat_h);
        let p = paint(&b, 60, 40, 2.0);
        for d in 0..DAYS {
            let band_top = f32::from(HEAT_TOP) + d as f32 * h;
            let any = p
                .iter()
                .any(|r| r.y >= band_top - 0.01 && r.y < band_top + h - 0.01);
            assert!(any, "day {d} has a band of cells");
        }
    }

    #[test]
    fn the_labels_name_every_row_of_the_grid() {
        let _g = crate::app::theme_test_guard();
        let c = cells(&one_hot(0, 0), 60, 40);
        let text = |row: u16| -> String {
            let mut v: Vec<_> = c.iter().filter(|c| c.row == row).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        // Each label is centred on the band it names, so a three-row day is
        // not a label with two unlabelled stripes under it.
        let h = layout(40).heat_h;
        let label = |d: u16| text(HEAT_TOP + d * h + (h - 1) / 2);
        assert!(label(0).starts_with("6d"), "{:?}", label(0));
        assert!(
            label(DAYS as u16 - 1).starts_with("now"),
            "the last band is now: {:?}",
            label(DAYS as u16 - 1)
        );
    }

    #[test]
    fn a_narrow_or_short_pane_draws_nothing_rather_than_a_mess() {
        let _g = crate::app::theme_test_guard();
        assert!(cells(&one_hot(0, 0), 20, 40).is_empty());
        assert!(paint(&one_hot(0, 0), 20, 40, 2.0).is_empty());
        assert!(cells(&one_hot(0, 0), 60, 4).is_empty());
    }

    #[test]
    fn money_never_rounds_a_real_cost_to_nothing() {
        assert_eq!(money(0), "$0.00");
        assert_eq!(money(1_500_000), "$1.50");
        // A session that cost a third of a cent is not free.
        assert_eq!(money(3_400), "$0.003");
    }

    /// The total written in the ring's hole must sit INSIDE the hole. It used
    /// to be centred on canvas column 6 while the ring was painted on a canvas
    /// shifted one column right of that, so at four characters — which is what
    /// `compact` is built to produce — its first one landed on the ring's left
    /// arc.
    #[test]
    fn the_total_sits_inside_the_hole_not_on_the_ring() {
        let _g = crate::app::theme_test_guard();
        let b = Buckets {
            hourly: vec![0; DAYS * HOURS],
            daily_cost: vec![0; DAYS],
            tok_in: 1_840_000,
            tok_out: 410_000,
            cost_microusd: 10,
        };
        let l = layout(40);
        let mut hole: Vec<(u16, char)> = cells(&b, 60, 40)
            .into_iter()
            .filter(|c| c.row == l.split_top + RING_ROW && c.col < 13)
            .map(|c| (c.col, c.c))
            .collect();
        hole.sort_by_key(|&(col, _)| col);
        let text: String = hole.iter().map(|&(_, c)| c).collect();
        assert_eq!(text, compact(2_250_000), "the total is in the hole");
        // The centre is measured off the RING ITSELF — the horizontal extent
        // of what `paint` actually emitted in the band — rather than read back
        // out of the same constant the text was placed from. Both ends
        // agreeing with one constant is not the property under test; both ends
        // agreeing with EACH OTHER is, and that is what a stray shift breaks.
        let band: Vec<_> = paint(&b, 60, 40, 2.0)
            .into_iter()
            .filter(|p| p.y >= f32::from(l.split_top) && p.y < f32::from(l.cost_top))
            .collect();
        assert!(!band.is_empty(), "the ring drew something to measure");
        let left = band.iter().fold(f32::MAX, |a, p| a.min(p.x));
        let right = band.iter().fold(0.0f32, |a, p| a.max(p.x + p.w));
        let centre = (left + right) / 2.0;
        // A cell is claimed from its left edge, so the character at `col`
        // spans `col..col+1`; both of its edges must clear the inner wall.
        for &(col, _) in &hole {
            let lo = f32::from(col) - centre;
            let hi = lo + 1.0;
            assert!(
                lo.abs() < RING_R_IN && hi.abs() < RING_R_IN,
                "column {col} of {text:?} is on the ring, not in its hole: \
                 the ring runs {left}..{right}, so the hole spans {RING_R_IN} \
                 either side of {centre}"
            );
        }
    }

    /// The ring is nearly two rows tall in each direction. Painted from the
    /// legend's own row it put its top arc through the word TOKENS; it starts
    /// on the row below now, and the band grew a row to hold it.
    #[test]
    fn the_ring_never_reaches_the_legend_that_names_it() {
        let _g = crate::app::theme_test_guard();
        let b = Buckets {
            hourly: vec![0; DAYS * HOURS],
            daily_cost: vec![0; DAYS],
            tok_in: 3,
            tok_out: 1,
            cost_microusd: 10,
        };
        let l = layout(40);
        for aspect in [1.6f32, 2.0, 2.4] {
            let ring: Vec<_> = paint(&b, 60, 40, aspect)
                .into_iter()
                .filter(|p| p.y >= f32::from(l.split_top) && p.y < f32::from(l.cost_top))
                .collect();
            assert!(!ring.is_empty(), "the ring drew something at {aspect}");
            let top = ring.iter().fold(f32::MAX, |a, p| a.min(p.y));
            assert!(
                top >= f32::from(l.split_top + 1),
                "the ring is on the legend row at aspect {aspect}: top {top}"
            );
            let bottom = ring.iter().fold(0.0f32, |a, p| a.max(p.y + p.h));
            assert!(
                bottom <= f32::from(l.split_top + SPLIT_ROWS) + 1e-3,
                "and stays in its band at aspect {aspect}: bottom {bottom}"
            );
        }
    }

    /// A tall pane spends its height on the charts rather than finishing 45%
    /// of the way down and leaving the rest paper — the same rule the left
    /// nav's own division follows.
    #[test]
    fn a_tall_pane_gives_its_slack_to_the_charts() {
        let short = layout(24);
        let tall = layout(56);
        assert!(
            tall.heat_h > short.heat_h,
            "the heatmap grew: {} vs {}",
            tall.heat_h,
            short.heat_h
        );
        assert!(
            tall.cost_rows > short.cost_rows,
            "and so did the cost curve: {} vs {}",
            tall.cost_rows,
            short.cost_rows
        );
        // …but neither runs away with it: a week of readings spread over
        // forty rows is a blob, not a shape.
        let huge = layout(200);
        assert_eq!(huge.heat_h, HEAT_ROW_MAX);
        assert_eq!(huge.cost_rows, COST_MAX);
    }

    /// Every band stays inside the pane, at every height a tile can be — a
    /// chart drawn past the last row is a chart drawn over the pane below it.
    #[test]
    fn no_band_is_ever_laid_out_past_the_last_row() {
        for rows in 0..120u16 {
            let l = layout(rows);
            assert!((1..=HEAT_ROW_MAX).contains(&l.heat_h), "{rows}: {l:?}");
            if l.cost_rows > 0 {
                assert!(l.cost_rows >= COST_MIN, "{rows}: {l:?}");
                // legend + chart + the axis row under it
                assert!(
                    l.cost_top + 1 + l.cost_rows < rows,
                    "{rows}: the axis row is off the pane: {l:?}"
                );
            }
        }
    }

    /// The cost band used to ask for five rows or nothing, so a quarter tile
    /// dropped it whole and left the rows it could not fill empty. It shrinks
    /// to its floor now and is only given up when even that will not fit.
    #[test]
    fn a_short_pane_shrinks_the_cost_band_before_it_drops_it() {
        // 22 rows is the pane's own floor: every band at its minimum, no
        // slack for anything to grow into.
        let l = layout(22);
        assert!(
            l.cost_rows > 0,
            "a 22-row pane still costs something: {l:?}"
        );
        assert_eq!(l.cost_rows, COST_MIN, "at its floor: {l:?}");
        // And a pane with no room under the donut at all gives it up rather
        // than drawing a one-row smear with an axis on top of it.
        assert_eq!(layout(l.cost_top + 2).cost_rows, 0);
    }

    /// Both ends of the division agree: the text under the cost curve sits on
    /// the row the curve actually stops at. They were two sums before.
    #[test]
    fn the_cost_axis_labels_sit_under_the_curve_they_label() {
        let _g = crate::app::theme_test_guard();
        let b = Buckets {
            hourly: vec![0; DAYS * HOURS],
            daily_cost: vec![1, 4, 2, 9, 3, 6, 5],
            tok_in: 10,
            tok_out: 5,
            cost_microusd: 100,
        };
        for rows in [24u16, 34, 56] {
            let l = layout(rows);
            let curve: Vec<_> = paint(&b, 60, rows, 2.0)
                .into_iter()
                .filter(|p| p.y >= f32::from(l.cost_top))
                .collect();
            assert!(!curve.is_empty(), "{rows} rows: the curve drew something");
            let bottom = curve.iter().fold(0.0f32, |a, p| a.max(p.y + p.h));
            let axis: Vec<_> = cells(&b, 60, rows)
                .into_iter()
                .filter(|c| c.row >= l.cost_top && c.c == '6')
                .map(|c| c.row)
                .collect();
            let axis_row = l.cost_top + 1 + l.cost_rows;
            assert_eq!(axis.first(), Some(&axis_row));
            // The curve's last quad is a fraction of a cell tall, so it stops
            // just inside the axis row rather than exactly on it — what must
            // not happen is stopping a whole row short, or running past.
            assert!(
                bottom <= f32::from(axis_row) + 1e-3 && bottom > f32::from(axis_row) - 1.0,
                "{rows} rows: the curve stops at {bottom}, the axis is on row {axis_row}"
            );
        }
    }

    #[test]
    fn compact_tokens_fit_in_a_donuts_hole() {
        assert_eq!(compact(0), "0");
        assert_eq!(compact(184_000), "184k");
        assert_eq!(compact(2_250_000), "2.2M");
        assert!(compact(u64::MAX / 2).len() <= 12);
    }
}
