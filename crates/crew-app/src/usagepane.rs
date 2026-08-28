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

/// Row the heatmap's first day sits on, and the rows each section owns.
const HEAT_TOP: u16 = 2;
const SPLIT_TOP: u16 = HEAT_TOP + DAYS as u16 + 2;
const SPLIT_ROWS: u16 = 5;
const COST_TOP: u16 = SPLIT_TOP + SPLIT_ROWS + 1;
const COST_ROWS: u16 = 5;
/// Columns of labels down the left of the heatmap (`Mon `), and the air kept
/// to the right of every chart.
const LABEL_W: u16 = 4;
const RIGHT_PAD: u16 = 2;

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

    // Heatmap: a row per day, an hour per column.
    if rows > HEAT_TOP + DAYS as u16 {
        for (i, label) in day_labels().into_iter().enumerate() {
            put(&mut out, &label, 1, HEAT_TOP + i as u16, t.text_muted);
        }
        // Hour ticks under the grid, at midnight / 06 / 12 / 18.
        let grid_w = cols.saturating_sub(LABEL_W + RIGHT_PAD);
        let hour_col = |h: usize| LABEL_W + (h as u16 * grid_w) / HOURS as u16;
        for h in [0usize, 6, 12, 18] {
            put(
                &mut out,
                &format!("{h:02}"),
                hour_col(h),
                HEAT_TOP + DAYS as u16,
                t.text_muted,
            );
        }
    }

    // The in/out donut's legend and the total in its hole.
    if rows > SPLIT_TOP + SPLIT_ROWS {
        put(&mut out, "TOKENS", 1, SPLIT_TOP, t.text_muted);
        let total = b.tok_in + b.tok_out;
        let hole = compact(total);
        let cx = 6.0f32;
        let start = (cx - hole.chars().count() as f32 / 2.0).round().max(0.0) as u16;
        put(&mut out, &hole, start, SPLIT_TOP + 2, t.ink);
        let pct = |v: u64| match total {
            0 => 0,
            _ => (v * 100 / total).min(100),
        };
        put(
            &mut out,
            &format!("in   {}  {}%", compact(b.tok_in), pct(b.tok_in)),
            13,
            SPLIT_TOP + 1,
            accent(),
        );
        put(
            &mut out,
            &format!("out  {}  {}%", compact(b.tok_out), pct(b.tok_out)),
            13,
            SPLIT_TOP + 3,
            t.ansi[13],
        );
    }

    // The daily-cost chart's label and the day it peaked.
    if rows > COST_TOP + COST_ROWS {
        put(&mut out, "COST PER DAY", 1, COST_TOP, t.text_muted);
        let peak = b.daily_cost.iter().copied().max().unwrap_or(0);
        put(
            &mut out,
            &format!("peak {}", money(peak)),
            cols.saturating_sub(14),
            COST_TOP,
            t.text_muted,
        );
        put(&mut out, "6d ago", 1, COST_TOP + COST_ROWS, t.text_muted);
        put(
            &mut out,
            "today",
            cols.saturating_sub(RIGHT_PAD + 5),
            COST_TOP + COST_ROWS,
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

    // Heatmap. Cold cells keep a faint trace so the grid reads as a grid; hot
    // ones walk the theme's own gradient, the same ramp the meters use.
    if rows > HEAT_TOP + DAYS as u16 {
        let mut c = Canvas::new(grid_w, DAYS as u16, aspect);
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

    // In / out donut.
    if rows > SPLIT_TOP + SPLIT_ROWS {
        let mut c = Canvas::new(12, SPLIT_ROWS, aspect);
        let (_, h) = c.size();
        let slices = [
            Slice::new(b.tok_in as f32, accent()),
            Slice::new(b.tok_out as f32, t.ansi[13]),
        ];
        pie::donut(&mut c, (6.0, h / 2.0), 3.6, 2.2, &slices, t.border_normal);
        pie::dot(&mut c, (6.0, h / 2.0), 2.2, t.page_bg, 1.0);
        pie::dot(&mut c, (12.5, h / 2.0 - aspect), 0.34, accent(), 1.0);
        pie::dot(&mut c, (12.5, h / 2.0 + aspect), 0.34, t.ansi[13], 1.0);
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(1.0, f32::from(SPLIT_TOP))),
        );
    }

    // Cost per day.
    if rows > COST_TOP + COST_ROWS {
        let peak = b.daily_cost.iter().copied().max().unwrap_or(0).max(1);
        let samples: Vec<f32> = b
            .daily_cost
            .iter()
            .map(|&v| (v as f32 / peak as f32).clamp(0.0, 1.0))
            .collect();
        let w_cells = cols.saturating_sub(2 + RIGHT_PAD);
        let mut c = Canvas::new(w_cells, COST_ROWS - 1, aspect);
        let (w, h) = c.size();
        area::draw(&mut c, (0.0, 0.0, w, h), &samples, t.ansi[11]);
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(1.0, f32::from(COST_TOP + 1))),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{cells, compact, money, paint, HEAT_TOP, LABEL_W};
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
        for (day, hour) in [(0usize, 0usize), (3, 12), (DAYS - 1, HOURS - 1)] {
            let p = paint(&one_hot(day, hour), 60, 40, 2.0);
            let hot = p
                .iter()
                .filter(|r| r.y >= f32::from(HEAT_TOP) && r.y < f32::from(HEAT_TOP + DAYS as u16))
                .max_by(|a, b| a.alpha.total_cmp(&b.alpha))
                .expect("the heatmap drew");
            let row = (hot.y - f32::from(HEAT_TOP)).floor() as usize;
            assert_eq!(row, day, "day {day} sits on its own row");
            // Its column: the grid spans LABEL_W..cols-RIGHT_PAD.
            let grid_w = 60.0 - f32::from(LABEL_W) - 2.0;
            let col = ((hot.x - f32::from(LABEL_W)) / grid_w * HOURS as f32).floor() as usize;
            assert_eq!(col, hour, "hour {hour} sits in its own column");
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
        let p = paint(&b, 60, 40, 2.0);
        for d in 0..DAYS {
            let row_top = f32::from(HEAT_TOP) + d as f32;
            let any = p
                .iter()
                .any(|r| r.y >= row_top - 0.01 && r.y < row_top + 0.99);
            assert!(any, "day {d} has a row of cells");
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
        assert!(text(HEAT_TOP).starts_with("6d"), "{:?}", text(HEAT_TOP));
        assert!(
            text(HEAT_TOP + DAYS as u16 - 1).starts_with("now"),
            "the last row is now: {:?}",
            text(HEAT_TOP + DAYS as u16 - 1)
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

    #[test]
    fn compact_tokens_fit_in_a_donuts_hole() {
        assert_eq!(compact(0), "0");
        assert_eq!(compact(184_000), "184k");
        assert_eq!(compact(2_250_000), "2.2M");
        assert!(compact(u64::MAX / 2).len() <= 12);
    }
}
