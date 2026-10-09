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
pub(crate) use crate::usagelayout::*;
use crew_render::{CellView, Paint};

use crate::palette::accent;
use crate::plot::pie::{self, Slice};
use crate::plot::{heatmap, Canvas};
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

/// Below this width the ring's key is `in 81%` and the cost chart has no
/// `peak`: `COST PER DAY` fills columns 1–12 and a peak put at `cols - 14`
/// landed on the same squares; a key cut mid-number is not a key.
const FULL_COLS: u16 = 28;

impl UsagePane {
    /// What this pane says in one line, for a thumbnail of it: the week's
    /// spend, which is the number the pane exists to show.
    pub(crate) fn glance(&self) -> String {
        format!("{} \u{b7} 7 days", money(self.buckets.cost_microusd))
    }

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

pub fn cells(b: &Buckets, cols: u16, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if cols < 24 || rows < 6 {
        return crate::toosmall::note(cols, rows);
    }
    // One column of air at the right edge; the cut is marked, not silent.
    let put = |out: &mut Vec<CellView>, s: &str, col: u16, row: u16, fg: (u8, u8, u8)| {
        crate::navtext::put_at(out, s, col, row, cols - 1, fg);
    };

    // Header: the week's totals, which is what the charts below are of —
    // on the first row, with no `USAGE` rule over them: the card's legend
    // already names the pane.
    put(
        &mut out,
        &crate::usagelayout::week_line(b, " \u{00b7} "),
        1,
        0,
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
        crate::usageaxis::hour_ticks(&mut out, LABEL_W, grid_w, heat_end, cols);
    }

    // The in/out donut's legend and the total in its hole.
    if rows > l.split_top + SPLIT_ROWS {
        put(&mut out, "TOKENS", 1, l.split_top, t.text_muted);
        let (hole, start, _) = hole(b);
        put(&mut out, &hole, start, l.split_top + RING_ROW, t.ink);
        // Floored on their own, the two read `81% / 18%` — see `usageaxis`.
        let (pct_in, pct_out) = crate::usageaxis::split_pct(b.tok_in, b.tok_out);
        // The two readings flank the ring's own centre row, so each sits
        // opposite the arc it is naming. Each is written in its slice's
        // colour: the words ARE the key, which is why the ring no longer
        // carries a pair of swatch dots beside it.
        let key = |name: &str, v: u64, pct: u64| match cols >= FULL_COLS {
            true => format!("{name:<4} {}  {pct}%", compact(v)),
            false => format!("{name} {pct}%"),
        };
        let (r_in, r_out) = (l.split_top + RING_ROW - 1, l.split_top + RING_ROW + 1);
        put(&mut out, &key("in", b.tok_in, pct_in), 13, r_in, accent());
        put(
            &mut out,
            &key("out", b.tok_out, pct_out),
            13,
            r_out,
            t.ansi[13],
        );
    }

    // The daily-cost chart's label and the day it peaked.
    if l.cost_rows > 0 {
        let axis = l.cost_top + 1 + l.cost_rows;
        put(&mut out, "COST PER DAY", 1, l.cost_top, t.text_muted);
        let peak = b.daily_cost.iter().copied().max().unwrap_or(0);
        let w = cols.saturating_sub(2 + RIGHT_PAD);
        let said = crate::costbars::peak_labelled(&b.daily_cost, w, l.cost_rows);
        if cols >= FULL_COLS && peak > 0 && !said {
            let peak = format!("peak {}", money(peak));
            put(&mut out, &peak, cols - 14, l.cost_top, t.text_muted);
        }
        let top = l.cost_top + 1;
        crate::costbars::labels(&mut out, &b.daily_cost, 1, w, top, l.cost_rows, axis, cols);
    }
    out
}

/// The total written in the ring's hole, the column it starts at, and the
/// centre it reads at. A cell-placed word can only centre to half a column,
/// so the ring centres on the WORD: on `RING_CX` itself a one-digit `0` sat
/// half a column right of the ring's middle.
fn hole(b: &Buckets) -> (String, u16, f32) {
    let text = compact(b.tok_in + b.tok_out);
    let n = text.chars().count() as f32;
    let start = (RING_CX - n / 2.0).round().max(0.0);
    (text, start as u16, start + n / 2.0)
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
        // Centred on the total in its hole, which sits on whole cells.
        let centre = (hole(b).2 - SHIFT, h / 2.0);
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
        // off the page rather than off the ring's inner edge. Not on glass:
        // there the page is see-through, and a solid page-coloured disc was a
        // puck over the desktop.
        if t.liquid.is_none() {
            pie::dot(&mut c, centre, RING_R_IN, t.page_bg, 1.0);
        }
        out.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(SHIFT, f32::from(l.split_top + 1))),
        );
    }

    // Cost per day, over whatever rows the division left it.
    if l.cost_rows > 0 {
        let w = cols.saturating_sub(2 + RIGHT_PAD);
        let bars = crate::costbars::paint(&b.daily_cost, w, l.cost_rows, aspect);
        out.extend(
            bars.into_iter()
                .map(|p| p.shifted(1.0, f32::from(l.cost_top + 1))),
        );
    }
    out
}

#[cfg(test)]
#[path = "usagepane_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "usagenarrow_tests.rs"]
mod narrow_tests;

#[cfg(test)]
#[path = "usagehole_tests.rs"]
mod hole_tests;
