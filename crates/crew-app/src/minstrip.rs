//! Minimized pane thumbnails: the bottom strip of fieldset cards for panes
//! demoted out of the full grid (LRU). Each card shows the pane title and a
//! marker — the attention glyph when the pane needs you, else the quiet
//! activity dot — enough to track a pane at a glance and click to restore it.
use crew_render::{CellView, PaneScene};

use crate::attention::Attention;
use crate::pane::Pane;
use crate::panelcard::push_card;

/// One full pulse of a busy pane's nav dot, in ms.
const PULSE_MS: u64 = 1100;

/// The one-cell marker for a thumbnail. Priority: an attention glyph (bell
/// colour, blinking on the shared clock) supersedes everything; else a busy
/// pane shows a **pulsing** dot (brightness bounces so you can see it working
/// at a glance); else a pane with recent activity shows a **steady** dot; else
/// nothing. A marker in its blink-off phase draws nothing, like the nav rows.
pub fn strip_marker(
    activity: bool,
    attention: Option<Attention>,
    busy: bool,
    now: u64,
) -> Option<(char, (u8, u8, u8))> {
    let t = crew_theme::theme();
    if let Some(a) = attention {
        return a.visible(now).then(|| (a.glyph(), t.bell));
    }
    if busy {
        // Pulse between a dim floor and full activity — always visible, never
        // fully off, so a busy pane reads as alive rather than blinking out.
        let floor = crate::anim::lerp_rgb(t.activity, t.page_bg, 0.6);
        let fg = crate::anim::lerp_rgb(floor, t.activity, crate::anim::tri(now, PULSE_MS));
        return Some((crate::shapecues::dot(true), fg));
    }
    activity.then_some((crate::shapecues::dot(false), t.activity))
}

/// The thumbnail's one content row: the marker on the left, and on the right
/// how many lines arrived while the pane was out of the grid.
///
/// The strip is where a pane goes when it has not been touched for a while,
/// which is exactly where the question "what did I miss?" is loudest — and
/// the marker alone could only ever answer "something".
pub fn strip_row(cols: u16, marker: Option<(char, (u8, u8, u8))>, unread: usize) -> Vec<CellView> {
    let mut v = Vec::new();
    let bg = crew_theme::theme().page_bg;
    if let Some((c, fg)) = marker {
        if cols > 0 {
            v.push(CellView {
                col: 0,
                row: 0,
                c,
                fg,
                bg,
                ..Default::default()
            });
        }
    }
    // The count needs a column of air after the marker, or a one-cell card
    // would draw the two on top of each other.
    if let Some(n) = crate::unread::badge(unread) {
        let w = n.chars().count() as u16;
        if cols > w + 1 {
            for (i, ch) in n.chars().enumerate() {
                v.push(CellView {
                    col: cols - w + i as u16,
                    row: 0,
                    c: ch,
                    fg: crew_theme::theme().activity,
                    bg,
                    bold: true,
                    ..Default::default()
                });
            }
        }
    }
    v
}

/// The `+N` tile's contents: the panes standing behind it, numbered the way
/// every other pane is, and — when there are more than the tile has rows —
/// a last line saying how many it could not name.
///
/// It used to `take(rows)` and `take(cols)` and say nothing about either. A
/// tile whose entire job is to answer "which panes are behind this?" was
/// dropping the tail of that answer twice over: a long title read `8 crew ·
/// claude-opus-5 revi`, and a sixth pane behind a four-row tile simply was
/// not there, under a legend that said `+6`.
pub fn overflow_cells(names: &[String], cols: u16, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut v = Vec::new();
    if cols == 0 || rows == 0 {
        return v;
    }
    // When the list does not fit, the last row is spent saying so — which is
    // worth more than one more name the reader cannot tell is the last.
    let rows = usize::from(rows);
    let shown = match names.len() > rows {
        true => rows - 1,
        false => names.len(),
    };
    let mut put = |row: usize, text: &str, number: bool| {
        for (i, c) in crate::chatwidth::clip_w(text, usize::from(cols))
            .chars()
            .enumerate()
        {
            // The leading number is how `Cmd+N` reaches the pane, so it wears
            // the accent — the actionable half of the row, marked the way the
            // welcome hint's chords and the `/keys` column are.
            let digits = number && text[..].chars().take(i + 1).all(|c| c.is_ascii_digit());
            v.push(CellView {
                col: i as u16,
                row: row as u16,
                c,
                fg: if digits {
                    crate::palette::accent()
                } else {
                    t.text_muted
                },
                bg: t.page_bg,
                ..Default::default()
            });
        }
    };
    for (row, name) in names.iter().take(shown).enumerate() {
        put(row, name, true);
    }
    if shown < names.len() {
        put(shown, &format!("+{} more", names.len() - shown), false);
    }
    v
}

/// Push one fieldset card per minimized pane into `scenes` — plus, when the
/// strip overflowed, a trailing `+N` card standing in for the panes it had no
/// readable room for (they stay listed in the sidebar's PANES section).
pub fn push_min_strip(
    scenes: &mut Vec<PaneScene>,
    panes: &[Pane],
    placed: &crate::grid::GridRects,
    cw: f32,
    ch: f32,
    hidden: &[usize],
) {
    if let Some((n, rect)) = placed.overflow {
        // `+3` says how many are behind the tile and nothing about which,
        // which is the one thing you would look at it to find out. The
        // numbers are the same ones `Cmd+N` uses.
        let names: Vec<String> = hidden
            .iter()
            .filter(|i| **i < panes.len())
            .map(|&i| format!("{} {}", i + 1, panes[i].title_text()))
            .collect();
        push_card(scenes, rect, cw, ch, &format!("+{n}"), move |cols, rows| {
            overflow_cells(&names, cols, rows)
        });
    }
    let now = crate::anim::now_ms();
    for &(idx, rect) in &placed.minimized {
        let Some(p) = panes.get(idx) else { continue };
        // Numbered like the full tiles are, because `Cmd+N` reaches a
        // minimized pane too and the number is how you know which N.
        let title = format!("{} {}", idx + 1, p.title_text());
        let marker = strip_marker(p.activity, p.attention, crate::paneview::pane_busy(p), now);
        let unread = match &p.content {
            crate::pane::PaneContent::Terminal(t) => {
                crate::unread::count(t.pty.scrollable_lines(), t.read_at)
            }
            _ => 0,
        };
        push_card(scenes, rect, cw, ch, &title, move |cols, _rows| {
            strip_row(cols, marker, unread)
        });
    }
}

#[cfg(test)]
#[path = "minstrip_tests.rs"]
mod tests;
