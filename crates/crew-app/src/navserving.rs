//! The SERVING card: who answers smith work, and how much of the 5h and 7d
//! windows is spent — the footer's first two lines, standing in the nav
//! where they are visible from any pane. Today's whole question ("why does
//! it say qwen-max?") is answered by the first line.
use crew_render::CellView;

use crate::navglance::Serving;
use crate::palette::accent;
use crate::summaryfit::{bar, fmt_left};

/// Rows the card occupies: rule, who, 5h, 7d, and a one-row gap.
pub const SERVING_BLOCK: u16 = 5;

/// Column the text starts on, under the rule's own indent (as GIT does).
const TEXT_COL: u16 = 3;

/// The card's cells and its meters' fractions, in row order — the paint
/// pass turns the `▓░` runs into drawn capsules (`summarymeter::draw_meters`)
/// exactly as the footer does.
pub(crate) fn serving_cells(s: &Serving, cols: u16) -> (Vec<CellView>, Vec<f32>) {
    let t = crew_theme::theme();
    let mut out =
        crate::boxdraw::section_header("SERVING", cols, t.border_normal, accent(), t.page_bg);
    let mut meters = Vec::new();
    if cols < 8 {
        return (out, meters);
    }
    let max_col = cols.saturating_sub(1);
    let room = usize::from(max_col.saturating_sub(TEXT_COL));
    // Row 1: provider · model — the provider in the accent, the model in ink.
    let who: Vec<(char, (u8, u8, u8))> = match (&s.provider, &s.model) {
        (None, None) => "no provider \u{2014} /model"
            .chars()
            .map(|c| (c, t.text_muted))
            .collect(),
        (p, m) => {
            let p = p.as_deref().unwrap_or("");
            let m = m.as_deref().unwrap_or("");
            let sep = if !p.is_empty() && !m.is_empty() {
                " \u{00b7} "
            } else {
                ""
            };
            let full = format!("{p}{sep}{m}");
            // The model is what the card exists to show: when the pair does
            // not fit, the provider goes and the model stays (see `/model`).
            let (shown, head) = if crate::chatwidth::str_w(&full) <= room || m.is_empty() {
                (full, p.chars().count())
            } else {
                (m.to_string(), 0)
            };
            crate::chatwidth::clip_w(&shown, room)
                .chars()
                .enumerate()
                .map(|(i, c)| (c, if i < head { accent() } else { t.ink }))
                .collect()
        }
    };
    put(&mut out, 1, who, max_col, t.page_bg);
    for (row, label, w) in [(2, "5h", s.windows.five_h), (3, "7d", s.windows.seven_d)] {
        let (pct, left) = match w {
            Some(w) => (
                ((w.spent.saturating_mul(100)) / w.budget.max(1)).min(100) as u8,
                fmt_left(w.left_ms),
            ),
            None => (0, "\u{2014}".to_string()),
        };
        meters.push(f32::from(pct) / 100.0);
        // The countdown goes before the bar does: a clipped "2…" says nothing.
        let mut line = format!("{label} {} {left}", bar(pct));
        if crate::chatwidth::str_w(&line) > room {
            line = format!("{label} {}", bar(pct));
        }
        let clipped = crate::chatwidth::clip_w(&line, room);
        let styled = clipped.chars().enumerate().map(|(i, c)| {
            let fg = if i < 2 { t.text_muted } else { t.ink };
            (c, fg)
        });
        put(&mut out, row, styled, max_col, t.page_bg);
    }
    (out, meters)
}

fn put(
    out: &mut Vec<CellView>,
    row: u16,
    styled: impl IntoIterator<Item = (char, (u8, u8, u8))>,
    max_col: u16,
    bg: (u8, u8, u8),
) {
    crate::chatwidth::place_row(TEXT_COL, max_col, styled, |x, c, fg| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            ..Default::default()
        });
    });
}

#[cfg(test)]
#[path = "navserving_tests.rs"]
mod tests;
