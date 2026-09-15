//! The rail's readings, one row builder each: the three system meters, the
//! two net rates, and what git has to say. Split from [`crate::navrailfoot`],
//! which decides which of them fit and where they sit.
use crew_render::{CellView, Paint};

use crate::git::GitInfo;
use crate::stats::Stats;

/// CPU, memory, disk: a letter and a drawn capsule, one row each. The number
/// is what gives at this width — a meter says "how full" in one look, and the
/// tier colour (the gauges' own) says which band it is in twice over.
pub(crate) fn sys(
    cells: &mut Vec<CellView>,
    paint: &mut Vec<Paint>,
    s: Stats,
    top: u16,
    cols: u16,
    aspect: f32,
) {
    let muted = crew_theme::theme().text_muted;
    for (i, (label, frac)) in [('C', s.cpu), ('M', s.mem), ('D', s.disk)]
        .into_iter()
        .enumerate()
    {
        let row = top + i as u16;
        crate::navtext::put_at(cells, &label.to_string(), 0, row, cols, muted);
        // From just past the letter to just short of the right edge: the
        // capsule's end caps want that air, and a meter flush to the card's
        // rule reads as part of the frame.
        let (x, w) = (1.4, f32::from(cols) - 1.8);
        if w <= 0.0 {
            continue;
        }
        let mut c = crate::plot::Canvas::new(cols, 1, aspect);
        crate::plot::meter::capsule(
            &mut c,
            x,
            0.0,
            w,
            aspect,
            frac,
            |_| crate::gauges::fill_color(frac),
            crate::gauges::track_color(),
        );
        paint.extend(
            c.paint()
                .into_iter()
                .map(|p| p.shifted(0.0, f32::from(row))),
        );
    }
}

/// The sky in five columns: the same WMO glyph the weather strip uses, and
/// this hour's temperature. The high, the low and the chance of rain are the
/// strip's, and they stay there — a rail-sized forecast is a wrong one.
pub(crate) fn sky(w: &crate::navweather::Weather) -> String {
    format!("{}{}\u{00b0}", crate::navweather::glyph(w.code), w.temp)
}

/// A per-second byte rate in four columns: the arrow beside it needs the
/// fifth. `12K`, `1.2M` — the unit is a letter because "KB/s" is the whole
/// rail.
pub(crate) fn short_rate(b: u64) -> String {
    const K: f64 = 1024.0;
    let f = b as f64;
    if f < 1000.0 {
        format!("{b}B")
    } else if f < 999.5 * K {
        format!("{:.0}K", f / K)
    } else if f < 9.95 * K * K {
        format!("{:.1}M", f / (K * K))
    } else {
        format!("{:.0}M", f / (K * K))
    }
}

/// Down then up, each on its own row: two rates on one five-column row would
/// be two clipped rates, and a clipped rate is a different number.
pub(crate) fn net(cells: &mut Vec<CellView>, rx: u64, tx: u64, top: u16, cols: u16) {
    let ink = crew_theme::theme().ink;
    for (i, (glyph, bytes, fg)) in [
        ('\u{2193}', rx, crate::net::spark()),
        ('\u{2191}', tx, crate::net::up_color()),
    ]
    .into_iter()
    .enumerate()
    {
        let row = top + i as u16;
        crate::navtext::put_at(cells, &glyph.to_string(), 0, row, cols, fg);
        crate::navtext::put_at(cells, &short_rate(bytes), 1, row, cols, ink);
    }
}

/// The first form that fits `cols` whole, or nothing at all.
///
/// A count clipped to its leading digits is not a smaller count, it is a
/// different one — `1234` changed files drawn as `● 123` is a lie the rail
/// would tell all day. Every ladder here ends in a rung that fits whatever it
/// is given, or says nothing.
fn pick(ladder: &[String], cols: u16) -> String {
    ladder
        .iter()
        .find(|s| crate::chatwidth::str_w(s) <= usize::from(cols))
        .cloned()
        .unwrap_or_default()
}

/// The tree, in the GIT card's own two marks: `● N` changed in the status
/// colour, or a muted `✓`, with the `↑↓` under it when there is any. The
/// branch name is the one thing here that cannot be clipped into something
/// true, so it is not shown at all — the rail is one click from the card
/// that spells it out.
pub(crate) fn git(cells: &mut Vec<CellView>, info: Option<&GitInfo>, top: u16, cols: u16) {
    let t = crew_theme::theme();
    let Some(i) = info else {
        return;
    };
    let (head, fg) = match i.changed > 0 {
        true => (
            pick(
                &[
                    format!("\u{25cf} {}", i.changed),
                    format!("\u{25cf}{}", i.changed),
                    "\u{25cf}\u{2026}".to_string(),
                ],
                cols,
            ),
            t.status_fg,
        ),
        false => ("\u{2713}".to_string(), t.text_muted),
    };
    crate::navtext::put_at(cells, &head, 0, top, cols, fg);
    let parts: Vec<String> = [(i.ahead, '\u{2191}'), (i.behind, '\u{2193}')]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, g)| format!("{g}{n}"))
        .collect();
    let Some(first) = parts.first() else {
        return;
    };
    // Both directions with air, both tight, then the one that is ahead — the
    // same giving-way the open nav's GIT rule does.
    let tail = pick(&[parts.join(" "), parts.concat(), first.clone()], cols);
    crate::navtext::put_at(cells, &tail, 0, top + 1, cols, t.ink);
}

#[cfg(test)]
#[path = "navrailrow_tests.rs"]
mod tests;
