//! `/dash` — one screen of everything crew knows about the machine it is on.
//!
//! The sidebar has answered these questions for a while, in a 22-column strip
//! where each answer gets one or two rows. Given a whole pane, the same
//! readings can be *drawn* at a size worth looking at: three ring gauges, a
//! CPU curve with room to have a shape, both directions of the network on one
//! axis, a week of token usage as a heatmap, and what each day of it cost.
//!
//! Nothing here is new data. It is the widgets built over the last ten
//! releases, composed — which is the point: they were built to be composed.
use crew_render::{CellView, Paint};

use crate::palette::accent;
use crate::plot::Canvas;
use crate::spark::History;
use crate::stats::SysSampler;
use crate::usageledger::Buckets;

pub struct DashPane {
    sampler: SysSampler,
    cpu: History,
    buckets: Buckets,
    /// Wall-clock ms of the last usage re-bucket — the ledger moves far more
    /// slowly than the sampler and is read on its own clock.
    usage_at: u64,
}

/// How often the usage buckets are re-read.
const USAGE_MS: u64 = 5_000;

impl DashPane {
    pub fn new() -> Self {
        Self {
            sampler: SysSampler::new(),
            cpu: History::new(240),
            buckets: crate::usageledger::buckets(wall_ms()),
            usage_at: crate::anim::now_ms(),
        }
    }

    /// Returns true when something moved and the pane should repaint.
    pub fn poll(&mut self) -> bool {
        let mut changed = self.sampler.refresh();
        if changed {
            let cpu = (self.sampler.stats().cpu.clamp(0.0, 1.0) * 100.0).round() as u64;
            self.cpu.push(cpu);
        }
        let now = crate::anim::now_ms();
        if now.saturating_sub(self.usage_at) >= USAGE_MS {
            self.usage_at = now;
            let next = crate::usageledger::buckets(wall_ms());
            changed |= next != self.buckets;
            self.buckets = next;
        }
        changed
    }
}

impl Default for DashPane {
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

/// The dashboard's bands, top to bottom. The three above the history are
/// fixed — they draw a machine, and a machine's readings do not get truer with
/// more rows. A band is drawn only when the pane has the rows for it, and the
/// order is the priority: a short pane keeps the machine and loses the
/// history.
const SYS_TOP: u16 = 1;
const SYS_ROWS: u16 = crate::sysdials::DASH.rows;
const NET_TOP: u16 = SYS_TOP + SYS_ROWS + 1;
const NET_ROWS: u16 = 3;
const USE_TOP: u16 = NET_TOP + NET_ROWS + 1;
/// The two below it are the histories, and they DO get truer with more rows —
/// so they take the pane's slack, in this order. See [`layout`].
const HEAT_ROW_MAX: u16 = 3;
const COST_MIN: u16 = 3;
const COST_MAX: u16 = 12;

/// How the dashboard divides `rows` below the machine, for one frame — the one
/// derivation the text and the drawing both read.
///
/// The bands were four `const`s, which is one layout for every pane: on a full
/// window they finished 55% of the way down and left 340 pixels of paper under
/// them, with a week of hours drawn as a one-row strip and seven days of cost
/// as a three-row smear. Same division `usagepane` makes, for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    /// Rows one day of the heatmap claims.
    heat_h: u16,
    /// Row the cost curve starts on; its legend is on the row above.
    cost_top: u16,
    /// Chart rows the cost curve gets — `0` when the pane cannot hold it.
    cost_rows: u16,
}

fn layout(rows: u16) -> Layout {
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
const CURVE_MIN: u16 = 18;

/// Columns the dial block claims on the left of the SYSTEM band; the CPU
/// curve fills what is left.
///
/// The dials give width back rather than squeezing the curve out: in a
/// narrow dash they draw smaller and keep their block within what is left.
fn ring_w(cols: u16) -> u16 {
    crate::sysdials::DASH_COLS
        .min(cols.saturating_sub(CURVE_MIN))
        .max(crate::sysdials::MIN_COLS)
}
/// Minimum pane the dashboard will draw in at all.
const MIN_COLS: u16 = 46;

impl DashPane {
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        let mut out = Vec::new();
        if cols < MIN_COLS || rows < SYS_TOP + SYS_ROWS {
            return out;
        }
        let (host, uptime) = crate::host::host_strings();
        let (one, five, fifteen) = crate::load::load_avg();
        put(
            &mut out,
            &format!("{host}  \u{00b7}  {uptime}  \u{00b7}  load {one:.2} {five:.2} {fifteen:.2}",),
            1,
            0,
            t.ink,
            cols,
        );

        // SYSTEM: the three dials, plus the CPU curve's own label.
        // `ring_w`, not `cols`: the dash gives the dials their own block and
        // puts the CPU curve beside them, so they spread inside that width
        // rather than across the whole pane.
        out.extend(crate::sysdials::DASH.cells(self.sampler.stats(), ring_w(cols), SYS_TOP));
        if cols > ring_w(cols) + 12 {
            put(
                &mut out,
                "CPU \u{00b7} 4 min",
                ring_w(cols) + 1,
                SYS_TOP,
                t.text_muted,
                cols,
            );
        }

        if rows > NET_TOP + NET_ROWS {
            let s = self.sampler.stats();
            // One line for the band, not two: the section rule and the rates
            // would land on the same row, and the last writer would win.
            put(
                &mut out,
                &format!(
                    "NET  \u{00b7}  \u{2193} {}   \u{2191} {}",
                    crate::net::rate(s.net_rx),
                    crate::net::rate(s.net_tx)
                ),
                1,
                NET_TOP - 1,
                t.text_muted,
                cols.saturating_sub(2),
            );
        }

        let l = layout(rows);
        let heat_end = USE_TOP + crate::usageledger::DAYS as u16 * l.heat_h;
        if rows > heat_end {
            let b = &self.buckets;
            put(
                &mut out,
                &format!(
                    "USAGE  \u{00b7}  {}  \u{00b7}  {} in / {} out  \u{00b7}  7 days",
                    crate::usagepane::money(b.cost_microusd),
                    crate::usagepane::compact(b.tok_in),
                    crate::usagepane::compact(b.tok_out),
                ),
                1,
                USE_TOP - 1,
                t.text_muted,
                cols,
            );
            // Each label centred on the band it names: a three-row day must
            // not read as a label with two unlabelled stripes under it.
            for (i, label) in ["6d", "5d", "4d", "3d", "2d", "1d", "now"]
                .iter()
                .enumerate()
            {
                let row = USE_TOP + i as u16 * l.heat_h + (l.heat_h - 1) / 2;
                put(&mut out, label, 1, row, t.text_muted, cols);
            }
        }

        if l.cost_rows > 0 {
            let peak = self.buckets.daily_cost.iter().copied().max().unwrap_or(0);
            put(
                &mut out,
                &format!(
                    "COST PER DAY  \u{00b7}  peak {}",
                    crate::usagepane::money(peak)
                ),
                1,
                l.cost_top - 1,
                t.text_muted,
                cols,
            );
        }
        out
    }

    pub fn paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        let t = crew_theme::theme();
        let mut out = Vec::new();
        if cols < MIN_COLS || rows < SYS_TOP + SYS_ROWS {
            return out;
        }
        // The three dials, and the CPU curve beside them.
        let ring = ring_w(cols);
        out.extend(crate::sysdials::DASH.paint(self.sampler.stats(), ring, SYS_TOP, aspect));
        let curve_w = cols.saturating_sub(ring + 2);
        if curve_w > 8 && !self.cpu.is_empty() {
            let samples: Vec<f32> = self
                .cpu
                .tail(curve_w as usize * 2)
                .into_iter()
                .map(|v| (v as f32 / 100.0).clamp(0.0, 1.0))
                .collect();
            let mut c = Canvas::new(curve_w, SYS_ROWS - 1, aspect);
            let (w, h) = c.size();
            crate::plot::area::draw(&mut c, (0.0, 0.0, w, h), &samples, accent());
            out.extend(
                c.paint()
                    .into_iter()
                    .map(|p| p.shifted(f32::from(ring + 1), f32::from(SYS_TOP + 1))),
            );
        }

        // Both directions of the network, on one axis.
        if rows > NET_TOP + NET_ROWS {
            let (rx, tx) = self.sampler.net_dirs();
            out.extend(crate::nettwin::paint(
                rx,
                tx,
                cols,
                NET_TOP + 1,
                aspect,
                crate::net::spark(),
                crate::net::up_color(),
            ));
        }

        // A week of tokens by hour. The grid is always DAYS x HOURS cells;
        // what the pane's height buys is the rows each of them is drawn over.
        let l = layout(rows);
        let heat_rows = crate::usageledger::DAYS as u16 * l.heat_h;
        if rows > USE_TOP + heat_rows {
            let grid_w = cols.saturating_sub(6);
            let mut c = Canvas::new(grid_w, heat_rows, aspect);
            let (w, h) = c.size();
            crate::plot::heatmap::draw(
                &mut c,
                (0.0, 0.0, w, h),
                &self.buckets.hourly,
                crate::usageledger::DAYS,
                crate::usageledger::HOURS,
                0.12,
                &|k: f32| {
                    let color = crate::modernring::pole_mix(k).unwrap_or_else(accent);
                    (color, 0.10 + 0.90 * k.powf(0.6))
                },
            );
            out.extend(
                c.paint()
                    .into_iter()
                    .map(|p| p.shifted(4.0, f32::from(USE_TOP))),
            );
        }

        // What each day cost, over whatever rows the division left it.
        if l.cost_rows > 0 {
            let peak = self
                .buckets
                .daily_cost
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
                .max(1);
            let samples: Vec<f32> = self
                .buckets
                .daily_cost
                .iter()
                .map(|&v| (v as f32 / peak as f32).clamp(0.0, 1.0))
                .collect();
            let mut c = Canvas::new(cols.saturating_sub(2), l.cost_rows, aspect);
            let (w, h) = c.size();
            crate::plot::area::draw(&mut c, (0.0, 0.0, w, h), &samples, t.ansi[11]);
            out.extend(
                c.paint()
                    .into_iter()
                    .map(|p| p.shifted(1.0, f32::from(l.cost_top))),
            );
        }
        out
    }
}

#[cfg(test)]
impl DashPane {
    /// Fill the histories and the ledger buckets with a plausible session, so
    /// the shot harness draws a dashboard with something on it.
    pub(crate) fn seed_for_test(&mut self) {
        for i in 0..240u64 {
            let t = i as f32 / 40.0;
            self.cpu
                .push((28.0 + 22.0 * t.sin() + if i > 190 { 35.0 } else { 0.0 }) as u64);
        }
        let mut hourly = vec![0u64; crate::usageledger::DAYS * crate::usageledger::HOURS];
        for d in 0..crate::usageledger::DAYS {
            for h in 0..crate::usageledger::HOURS {
                let weekend = d == 2 || d == 3;
                hourly[d * crate::usageledger::HOURS + h] = match (weekend, (9..20).contains(&h)) {
                    (true, _) => 0,
                    (false, true) => 3_000 + ((d * 1_300 + h * 900) % 11_000) as u64,
                    (false, false) => ((h % 5) * 300) as u64,
                };
            }
        }
        self.buckets = Buckets {
            hourly,
            daily_cost: vec![130_000, 420_000, 0, 40_000, 900_000, 510_000, 280_000],
            tok_in: 1_920_000,
            tok_out: 430_000,
            cost_microusd: 2_280_000,
        };
    }
}

fn put(out: &mut Vec<CellView>, s: &str, col: u16, row: u16, fg: (u8, u8, u8), cols: u16) {
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

#[cfg(test)]
#[path = "dashpane_tests.rs"]
mod tests;
