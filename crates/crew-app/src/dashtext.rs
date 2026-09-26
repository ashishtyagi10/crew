//! The `/dash` pane's WORDS: the machine's line, each band's legend, the day
//! labels down the heatmap and the axes under the two charts. Split from
//! [`super`] (child module — it reads the pane's own fields) along the line
//! between what the dashboard says and what it draws, which is where the line
//! cap found a boundary when the charts gained their axes.
use super::*;

pub(super) fn cells(d: &DashPane, cols: u16, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if cols < MIN_COLS || rows < SYS_TOP + SYS_ROWS {
        return crate::toosmall::note(cols, rows);
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
    out.extend(crate::sysdials::DASH.cells(d.sampler.stats(), ring_w(cols), SYS_TOP));
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
        let s = d.sampler.stats();
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
        let b = &d.buckets;
        put(
            &mut out,
            &format!(
                "USAGE  \u{00b7}  {}",
                crate::usagelayout::week_line(b, "  \u{00b7}  ")
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
        // …and the hours across the bottom of them, the way `/usage`
        // names them: seven unlabelled bands cannot say whether a stripe
        // is your morning or your evening. The grid's own geometry —
        // inset four columns, two of air on the right (see `paint`).
        crate::usageaxis::hour_ticks(&mut out, 4, cols.saturating_sub(6), heat_end, cols);
    }

    if l.cost_rows > 0 {
        // A week with nothing spent has no peak: `peak $0.00` read as a
        // meter reading zero, the way the USAGE line did before it said so.
        let peak = d.buckets.daily_cost.iter().copied().max().unwrap_or(0);
        let head = match peak {
            0 => "COST PER DAY".to_string(),
            p => format!(
                "COST PER DAY  \u{00b7}  peak {}",
                crate::usagepane::money(p)
            ),
        };
        put(&mut out, &head, 1, l.cost_top - 1, t.text_muted, cols);
        // Which days those are. Bars with a peak and no dates under them
        // says something happened, not when.
        let axis = l.cost_top + l.cost_rows;
        if axis < rows {
            let (w, top) = (cols.saturating_sub(2), l.cost_top);
            let daily = &d.buckets.daily_cost;
            crate::costbars::labels(&mut out, daily, 1, w, top, l.cost_rows, axis, cols);
        }
    }
    out
}
