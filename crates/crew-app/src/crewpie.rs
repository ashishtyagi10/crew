//! The crew donut: the PANES section's one-glance answer to *what is the crew
//! doing right now* — a ring split into working / waiting / idle, the pane
//! count in its hole, and a legend beside it.
//!
//! It replaces a one-row pulse sparkline that could only say how many panes
//! were busy, in eight height levels, with no way to show what the rest were.
//! The pulse history did not go away: it is drawn as a faint fill behind the
//! legend, so the ring says the present and the wash behind it says the last
//! minute. Fill only, and only to the right of the ring — a stroke crossing a
//! word is a scribble however faint it is, and the donut is the one thing in
//! the block the eye is meant to land on.
use crew_render::{CellView, Paint};

use crate::palette::accent;
use crate::panelist::PaneRow;
use crate::plot::pie::{self, Slice};
use crate::plot::Canvas;

/// Rows the block occupies, under the PANES header.
pub const ROWS: u16 = 3;

/// Where the donut sits and how big it is, in canvas units (one unit = one
/// cell width). Deliberately small: the sidebar is narrow, and the ring has to
/// leave the legend enough columns to name its own colours.
const CENTRE_X: f32 = 3.3;
const R_OUT: f32 = 2.4;
const R_IN: f32 = 1.25;
/// First column of the legend — clear of the ring, with a column of air.
const LEGEND_COL: u16 = 7;
/// Where the pulse backdrop starts: past the ring, so the two never overlap.
const WASH_X: f32 = CENTRE_X + R_OUT + 0.4;

/// How the crew divides up right now. `waiting` is a pane that has raised a
/// marker for you (a bell, a finished command); `working` is one doing
/// background work; everything else is idle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Mix {
    pub waiting: usize,
    pub working: usize,
    pub idle: usize,
}

impl Mix {
    pub fn total(&self) -> usize {
        self.waiting + self.working + self.idle
    }
}

/// Sort the open panes into the three states. Attention outranks busy: a pane
/// that wants you is *waiting on you*, whatever else it is doing — the same
/// precedence the pane row's own marker slot uses.
pub fn mix(panes: &[PaneRow]) -> Mix {
    let mut m = Mix::default();
    for p in panes {
        if p.attention.is_some() {
            m.waiting += 1;
        } else if p.busy {
            m.working += 1;
        } else {
            m.idle += 1;
        }
    }
    m
}

/// `(label, count, colour)` per legend line, in ring order.
fn entries(m: &Mix) -> [(&'static str, usize, (u8, u8, u8)); 3] {
    let t = crew_theme::theme();
    [
        ("working", m.working, accent()),
        ("waiting", m.waiting, t.bell),
        ("idle", m.idle, t.text_muted),
    ]
}

/// The block's text: the total in the ring's hole, and the legend lines.
/// `row0` is the row the block starts on inside the card.
pub fn cells(m: &Mix, cols: u16, row0: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    // The count in the hole, centred on the ring's middle row. It is the one
    // number the whole widget is *about*, so it goes where the eye lands.
    let total = m.total().to_string();
    let start = (CENTRE_X - total.chars().count() as f32 / 2.0)
        .round()
        .max(0.0) as u16;
    for (i, ch) in total.chars().enumerate() {
        out.push(cell(start + i as u16, row0 + 1, ch, t.ink, t.page_bg));
    }
    if cols <= LEGEND_COL + 4 {
        return out; // too narrow for a legend: the ring alone still reads
    }
    let text_col = LEGEND_COL + 2; // the swatch dot owns the two columns left of it
    for (k, (label, n, fg)) in entries(m).into_iter().enumerate() {
        let row = row0 + k as u16;
        // A category with no members is dimmed rather than dropped: the legend
        // is a fixed key, and a key that reorders itself as the crew changes
        // is harder to read than one that stays put.
        let fg = if n == 0 { t.text_muted } else { fg };
        let budget = cols.saturating_sub(text_col + 3);
        for (i, ch) in label.chars().take(budget as usize).enumerate() {
            out.push(cell(text_col + i as u16, row, ch, fg, t.page_bg));
        }
        let count = n.to_string();
        let cx = cols.saturating_sub(1 + count.chars().count() as u16);
        for (i, ch) in count.chars().enumerate() {
            out.push(cell(cx + i as u16, row, ch, fg, t.page_bg));
        }
    }
    out
}

/// The block's drawing: the pulse wash, the ring, and the legend swatches.
/// `pulse` is the busy-pane history the old sparkline used.
pub fn paint(
    m: &Mix,
    cols: u16,
    row0: u16,
    aspect: f32,
    pulse: &crate::spark::History,
) -> Vec<Paint> {
    if cols == 0 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut c = Canvas::new(cols, ROWS, aspect);
    let (w, h) = c.size();

    // The last minute of crew workload, behind the legend: quiet enough to
    // read through (half the chart's own alpha — at full strength it fights
    // the text it sits under), present enough to see the shape of a swarm
    // that has since finished.
    let peak = pulse.peak(cols as usize).max(1);
    let samples: Vec<f32> = pulse
        .tail(cols as usize * 2)
        .into_iter()
        .map(|v| (v as f32 / peak as f32).clamp(0.0, 1.0))
        .collect();
    if !samples.is_empty() {
        let mut back = Canvas::new(cols, ROWS, aspect);
        // Fill only, and starting clear of the ring. With its stroke on, the
        // curve ran a faint line straight through "working" and "waiting" —
        // a scribble across the legend, not a backdrop behind it — and its
        // left end crossed the donut, which is the one thing in the block the
        // eye is meant to land on.
        crate::plot::area::draw_styled(
            &mut back,
            (WASH_X, 0.0, (w - WASH_X).max(0.0), h),
            &samples,
            accent(),
            crate::plot::area::Style::wash(),
        );
        for p in back.paint() {
            c.rect(p.x, p.y * aspect, p.w, p.h * aspect, p.color, p.alpha * 0.5);
        }
    }

    let centre = (CENTRE_X, h / 2.0);
    let slices = [
        Slice::new(m.working as f32, accent()),
        Slice::new(m.waiting as f32, t.bell),
        Slice::new(m.idle as f32, t.text_muted),
    ];
    pie::donut(&mut c, centre, R_OUT, R_IN, &slices, t.border_normal);
    // The hole is punched back out to the page so the number in it reads
    // against the page and not through the wash behind the block.
    pie::dot(&mut c, centre, R_IN, t.page_bg, 1.0);

    if cols > LEGEND_COL + 4 {
        for (k, (_, n, col)) in entries(m).into_iter().enumerate() {
            let cy = (k as f32 + 0.5) * aspect;
            let col = if n == 0 { t.border_normal } else { col };
            pie::dot(&mut c, (f32::from(LEGEND_COL) + 0.5, cy), 0.36, col, 1.0);
        }
    }
    c.paint()
        .into_iter()
        .map(|p| p.shifted(0.0, f32::from(row0)))
        .collect()
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{cells, mix, paint, Mix, ROWS};
    use crate::panelist::PaneRow;

    fn pane(busy: bool, attention: bool) -> PaneRow {
        PaneRow {
            index: 1,
            title: "x".into(),
            focused: false,
            activity: false,
            minimized: false,
            attention: attention.then_some(('!', true)),
            busy,
            unread: 0,
            hovered: false,
        }
    }

    #[test]
    fn a_pane_that_wants_you_is_waiting_whatever_else_it_is_doing() {
        // Same precedence as the row's own marker slot: attention outranks
        // busy, so a swarm that has raised a bell is counted once, as waiting.
        let m = mix(&[pane(true, true), pane(true, false), pane(false, false)]);
        assert_eq!(
            m,
            Mix {
                waiting: 1,
                working: 1,
                idle: 1
            }
        );
        assert_eq!(m.total(), 3);
    }

    #[test]
    fn the_hole_carries_the_pane_count_and_the_legend_carries_the_split() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 1,
            working: 12,
            idle: 0,
        };
        let out = cells(&m, 24, 0);
        let row = |r: u16| -> String {
            let mut v: Vec<_> = out.iter().filter(|c| c.row == r).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert!(row(1).starts_with("13"), "total in the hole: {:?}", row(1));
        assert!(row(0).contains("working") && row(0).ends_with("12"));
        assert!(row(1).contains("waiting") && row(1).ends_with('1'));
        assert!(row(2).contains("idle") && row(2).ends_with('0'));
    }

    #[test]
    fn an_empty_category_is_dimmed_not_dropped() {
        let _g = crate::app::theme_test_guard();
        let t = crew_theme::theme();
        let out = cells(
            &Mix {
                waiting: 0,
                working: 2,
                idle: 1,
            },
            24,
            0,
        );
        // The key stays in place — a legend that reorders itself as the crew
        // changes is harder to read than one that does not.
        let waiting: Vec<_> = out.iter().filter(|c| c.row == 1 && c.c == 'w').collect();
        assert!(!waiting.is_empty(), "the waiting line is still drawn");
        assert!(waiting.iter().all(|c| c.fg == t.text_muted));
    }

    #[test]
    fn a_narrow_sidebar_keeps_the_ring_and_drops_the_legend() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 0,
            working: 1,
            idle: 1,
        };
        let text: String = cells(&m, 9, 0).iter().map(|c| c.c).collect();
        assert_eq!(text, "2", "only the count in the hole survives");
        // The ring itself is still drawn: the widget degrades, it does not
        // disappear.
        let hist = crate::spark::History::new(8);
        assert!(!paint(&m, 9, 0, 2.0, &hist).is_empty());
    }

    #[test]
    fn the_ring_is_drawn_inside_the_rows_the_block_reserved() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 1,
            working: 1,
            idle: 1,
        };
        let hist = crate::spark::History::new(8);
        let p = paint(&m, 24, 7, 2.0, &hist);
        assert!(!p.is_empty());
        for r in &p {
            assert!(r.y >= 7.0, "starts at the block's first row: {r:?}");
            assert!(
                r.y + r.h <= 7.0 + f32::from(ROWS) + 1e-3,
                "and stays inside it: {r:?}"
            );
        }
    }

    /// The wash is a backdrop, and a backdrop that crosses the ring is a
    /// smudge on the one mark the block is about. Nothing it draws may reach
    /// the donut, and it may not draw a curve at all.
    #[test]
    fn the_pulse_wash_never_reaches_the_ring() {
        let _g = crate::app::theme_test_guard();
        let mut hist = crate::spark::History::new(64);
        for v in 0..40 {
            hist.push(v % 5);
        }
        let m = Mix::default(); // no ring of its own: every accent quad is wash
        let p = paint(&m, 24, 0, 2.0, &hist);
        let wash: Vec<_> = p
            .iter()
            .filter(|r| r.color == crate::palette::accent())
            .collect();
        assert!(!wash.is_empty(), "there is a wash to test");
        let ring_right = super::CENTRE_X + super::R_OUT;
        assert!(
            wash.iter().all(|r| r.x >= ring_right),
            "wash crosses the ring (right edge {ring_right}): {wash:?}"
        );
    }

    #[test]
    fn the_pulse_wash_stays_quiet_enough_to_read_through() {
        let _g = crate::app::theme_test_guard();
        let m = Mix::default();
        let mut hist = crate::spark::History::new(64);
        for v in 0..40 {
            hist.push(v % 5);
        }
        let p = paint(&m, 24, 0, 2.0, &hist);
        // The backdrop is texture, not a second chart competing with the
        // legend written over it.
        let loud = p.iter().filter(|r| r.alpha > 0.4).count();
        let quiet = p.iter().filter(|r| r.alpha <= 0.4).count();
        assert!(quiet > loud, "wash {quiet} quiet vs {loud} loud rectangles");
    }
}
