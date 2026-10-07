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
    // One separator for the whole line: the name and OS were joined with
    // ` · ` inside a line spaced `  ·  `, so `Mac · macOS  ·  up 3d` read as
    // two kinds of break.
    let (host, os, uptime) = crate::host::host_parts();
    let (one, five, fifteen) = crate::load::load_avg();
    let load = format!("load {one:.2} {five:.2} {fifteen:.2}");
    let parts = [host, os, uptime, load];
    let mut line: Vec<&str> = parts
        .iter()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .collect();
    // Shed whole parts from the end — the load, then the uptime — until the
    // line fits with a column of air each side: a quarter tile ended on
    // `up 20h 10m  ·  …`, a clip that kept the separator and lost the part.
    let room = usize::from(cols.saturating_sub(2));
    while line.len() > 1 && crate::chatwidth::str_w(&line.join("  \u{00b7}  ")) > room {
        line.pop();
    }
    put(&mut out, &line.join("  \u{00b7}  "), 1, 0, t.ink, cols);

    // SYSTEM: the three dials, plus the CPU curve's own label.
    // `ring_w`, not `cols`: the dash gives the dials their own block and
    // puts the CPU curve beside them, so they spread inside that width
    // rather than across the whole pane.
    out.extend(crate::sysdials::DASH.cells(d.sampler.stats(), ring_w(cols), SYS_TOP));
    // The span the curve actually draws — one sample a second, as many as
    // its width holds — not the history's capacity: `CPU · 4 min` sat over
    // eighty seconds of curve on a half tile, and over no curve at all on a
    // dash just opened.
    let drawn = d
        .cpu
        .len()
        .min(usize::from(cols.saturating_sub(ring_w(cols) + 2)) * 2);
    if cols > ring_w(cols) + 12 && drawn > 1 {
        let span = crate::runclock::ladder(drawn as u64);
        let label = format!("CPU \u{00b7} {span}");
        put(
            &mut out,
            &label,
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
        let (rx, tx) = d.sampler.net_dirs();
        if crate::nettwin::reading(rx, tx, cols).quiet {
            crate::net::caption(&mut out, NET_TOP + (NET_ROWS - 1) / 2, cols);
        }
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
        let (w, top) = (cols.saturating_sub(2), l.cost_top);
        let said = crate::costbars::peak_labelled(&d.buckets.daily_cost, w, l.cost_rows);
        let head = match peak {
            p if p == 0 || said => "COST PER DAY".to_string(),
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
            let daily = &d.buckets.daily_cost;
            crate::costbars::labels(&mut out, daily, 1, w, top, l.cost_rows, axis, cols);
        }
    }
    out
}

#[cfg(test)]
#[path = "dashlabel_tests.rs"]
mod label_tests;
