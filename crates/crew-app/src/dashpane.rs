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

/// The dashboard's bands, top to bottom, and the rows each claims. A band is
/// drawn only when the pane has the rows for it — the order is the priority,
/// so a short pane keeps the machine and loses the history.
const SYS_TOP: u16 = 1;
const SYS_ROWS: u16 = crate::sysdials::ROWS;
const NET_TOP: u16 = SYS_TOP + SYS_ROWS + 1;
const NET_ROWS: u16 = 3;
const USE_TOP: u16 = NET_TOP + NET_ROWS + 1;
const USE_ROWS: u16 = crate::usageledger::DAYS as u16 + 1;
const COST_TOP: u16 = USE_TOP + USE_ROWS + 1;
const COST_ROWS: u16 = 4;
/// Columns the ring block claims on the left of the SYSTEM band; the CPU
/// curve fills what is left.
const RING_W: u16 = 3 + 3 * 6;
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

        // SYSTEM: the three rings, plus the CPU curve's own label.
        // RING_W, not `cols`: the dash gives the rings a fixed block and puts
        // the CPU curve beside them, so they spread inside their own width
        // rather than across the whole pane.
        out.extend(crate::sysdials::cells(
            self.sampler.stats(),
            RING_W,
            SYS_TOP,
        ));
        if cols > RING_W + 12 {
            put(
                &mut out,
                "CPU \u{00b7} 4 min",
                RING_W + 1,
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

        if rows > USE_TOP + USE_ROWS {
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
            for (i, label) in ["6d", "5d", "4d", "3d", "2d", "1d", "now"]
                .iter()
                .enumerate()
            {
                put(&mut out, label, 1, USE_TOP + i as u16, t.text_muted, cols);
            }
        }

        if rows > COST_TOP + COST_ROWS {
            let peak = self.buckets.daily_cost.iter().copied().max().unwrap_or(0);
            put(
                &mut out,
                &format!(
                    "COST PER DAY  \u{00b7}  peak {}",
                    crate::usagepane::money(peak)
                ),
                1,
                COST_TOP - 1,
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
        // The three rings, and the CPU curve beside them.
        out.extend(crate::sysdials::paint(
            self.sampler.stats(),
            RING_W,
            SYS_TOP,
            aspect,
        ));
        let curve_w = cols.saturating_sub(RING_W + 2);
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
                    .map(|p| p.shifted(f32::from(RING_W + 1), f32::from(SYS_TOP + 1))),
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

        // A week of tokens by hour.
        if rows > USE_TOP + USE_ROWS {
            let grid_w = cols.saturating_sub(6);
            let mut c = Canvas::new(grid_w, crate::usageledger::DAYS as u16, aspect);
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

        // What each day cost.
        if rows > COST_TOP + COST_ROWS {
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
            let mut c = Canvas::new(cols.saturating_sub(2), COST_ROWS - 1, aspect);
            let (w, h) = c.size();
            crate::plot::area::draw(&mut c, (0.0, 0.0, w, h), &samples, t.ansi[11]);
            out.extend(
                c.paint()
                    .into_iter()
                    .map(|p| p.shifted(1.0, f32::from(COST_TOP))),
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
mod tests {
    use super::{DashPane, COST_TOP, MIN_COLS, NET_TOP, SYS_TOP, USE_TOP};

    /// The bands are drawn in priority order, so a pane that cannot hold the
    /// history still holds the machine. A dashboard that vanishes below some
    /// height is worse than one that says less.
    #[test]
    fn a_short_pane_keeps_the_machine_and_loses_the_history() {
        let _g = crate::app::theme_test_guard();
        let d = DashPane::new();
        let rows_of = |rows: u16| -> Vec<u16> {
            let mut v: Vec<u16> = d
                .cells(100, rows)
                .into_iter()
                .map(|c| c.row)
                .chain(
                    d.paint(100, rows, 2.0)
                        .into_iter()
                        .map(|p| p.y.floor() as u16),
                )
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let tall = rows_of(40);
        assert!(
            tall.iter().any(|&r| r >= COST_TOP),
            "the tall pane has every band"
        );
        let short = rows_of(NET_TOP + 1);
        assert!(!short.is_empty(), "the machine is still drawn");
        assert!(
            short.iter().all(|&r| r < USE_TOP),
            "a short pane drew a band it has no room for: {short:?}"
        );
    }

    #[test]
    fn a_narrow_pane_draws_nothing_rather_than_a_mess() {
        let _g = crate::app::theme_test_guard();
        let d = DashPane::new();
        assert!(d.cells(MIN_COLS - 1, 40).is_empty());
        assert!(d.paint(MIN_COLS - 1, 40, 2.0).is_empty());
    }

    #[test]
    fn every_band_stays_in_its_own_rows() {
        let _g = crate::app::theme_test_guard();
        let d = DashPane::new();
        // The rings own the SYSTEM band; nothing they draw may reach the NET
        // header a row below it.
        let rings = crate::sysdials::paint(d.sampler.stats(), 21, SYS_TOP, 2.0);
        for p in rings {
            assert!(p.y >= f32::from(SYS_TOP), "{p:?}");
            assert!(
                p.y + p.h <= f32::from(NET_TOP - 1) + 1e-3,
                "a ring reached the NET band: {p:?}"
            );
        }
    }

    #[test]
    fn the_dashboard_draws_something_on_a_real_pane() {
        let _g = crate::app::theme_test_guard();
        let mut d = DashPane::new();
        for v in 0..40u64 {
            d.cpu.push(v * 2 % 100);
        }
        assert!(!d.cells(110, 36).is_empty());
        assert!(!d.paint(110, 36, 2.0).is_empty(), "the widgets drew");
    }
}
