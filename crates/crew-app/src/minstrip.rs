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

/// The `+N` tile's contents: as many of the panes standing behind it as fit,
/// numbered the way every other pane is.
pub fn overflow_cells(names: &[String], cols: u16, rows: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut v = Vec::new();
    for (row, name) in names.iter().take(usize::from(rows)).enumerate() {
        for (i, c) in name.chars().take(usize::from(cols)).enumerate() {
            v.push(CellView {
                col: i as u16,
                row: row as u16,
                c,
                fg: t.text_muted,
                bg: t.page_bg,
                ..Default::default()
            });
        }
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
mod tests {
    use super::*;
    use crate::attention::{Attention, BLINK_MS};
    use crate::notify::NotifyKind;

    /// `+3` says how many panes are behind the tile and nothing about which,
    /// which is the one thing you would look at it to find out.
    #[test]
    fn the_overflow_tile_names_the_panes_behind_it() {
        let _g = crate::app::theme_test_guard();
        let names = vec!["7 build".to_string(), "8 crew \u{b7} claude".to_string()];
        let cells = overflow_cells(&names, 20, 4);
        let row = |r: u16| -> String {
            let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert_eq!(row(0), "7 build");
        assert!(row(1).starts_with("8 crew"), "{:?}", row(1));
    }

    /// A tile with room for two names shows two, and clips a long one rather
    /// than drawing past its own card.
    #[test]
    fn the_overflow_tile_shows_what_fits_and_no_more() {
        let _g = crate::app::theme_test_guard();
        let names: Vec<String> = (0..9)
            .map(|i| format!("{i} a-very-long-pane-name"))
            .collect();
        let cells = overflow_cells(&names, 8, 2);
        assert!(
            cells.iter().all(|c| c.col < 8 && c.row < 2),
            "drew past the card"
        );
        assert_eq!(cells.iter().filter(|c| c.row == 0).count(), 8);
    }

    /// A thumbnail is the narrowest card crew draws — the strip fits as many
    /// as it can — so it is where a marker and a count are likeliest to land
    /// on each other.
    #[test]
    fn a_thumbnail_never_draws_two_glyphs_in_one_cell() {
        let _g = crate::app::theme_test_guard();
        let marker = Some(('\u{25cf}', crew_theme::theme().activity));
        for cols in 0..=40u16 {
            for unread in [0usize, 7, 128] {
                let cells = strip_row(cols, marker, unread);
                assert!(
                    cells.iter().all(|c| c.col < cols.max(1)),
                    "{cols}: a cell escaped a thumbnail"
                );
                let mut used: Vec<u16> = cells.iter().map(|c| c.col).collect();
                used.sort_unstable();
                let before = used.len();
                used.dedup();
                assert_eq!(
                    used.len(),
                    before,
                    "{cols}/{unread}: two glyphs in one cell"
                );
            }
        }
    }

    #[test]
    fn attention_supersedes_the_activity_dot() {
        let _g = crate::app::theme_test_guard();
        let a = Attention {
            kind: NotifyKind::AgentDone,
            at_ms: 0,
        };
        let t = crew_theme::theme();
        assert_eq!(strip_marker(true, Some(a), false, 0), Some(('✓', t.bell)));
        assert_eq!(strip_marker(true, None, false, 0), Some(('●', t.activity)));
        assert_eq!(strip_marker(false, None, false, 0), None);
    }

    #[test]
    fn marker_blinks_off_mid_pulse() {
        let a = Attention {
            kind: NotifyKind::Bell,
            at_ms: 0,
        };
        assert_eq!(strip_marker(false, Some(a), false, BLINK_MS), None);
        assert!(strip_marker(false, Some(a), false, 2 * BLINK_MS).is_some());
    }

    /// The strip is where a pane goes when you have not looked at it, so it
    /// is where "how much did I miss" matters most. The count is right-
    /// aligned; the marker keeps the left.
    #[test]
    fn a_thumbnail_shows_the_count_of_what_arrived_while_it_was_away() {
        let _g = crate::app::theme_test_guard();
        let marker = Some(('\u{25cf}', crew_theme::theme().activity));
        let row = strip_row(12, marker, 7);
        let at = |col: u16| row.iter().find(|c| c.col == col).map(|c| c.c);
        assert_eq!(at(0), Some('\u{25cf}'), "the marker lost its column");
        assert_eq!(at(11), Some('7'), "the count is not at the right edge");
        let many = strip_row(12, marker, 4000);
        let text: String = {
            let mut v: Vec<&CellView> = many.iter().filter(|c| c.col > 0).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert_eq!(text, "99+", "{text:?}");
    }

    /// Nothing new, nothing drawn — and a card with no room for both keeps
    /// the marker, which is the one that says a pane is alive.
    #[test]
    fn a_quiet_or_tiny_thumbnail_draws_no_count() {
        let _g = crate::app::theme_test_guard();
        let marker = Some(('\u{25cf}', crew_theme::theme().activity));
        assert_eq!(strip_row(12, marker, 0).len(), 1);
        assert_eq!(
            strip_row(2, marker, 7).len(),
            1,
            "the count crowded out the marker"
        );
        assert!(strip_row(0, marker, 7).is_empty());
    }

    #[test]
    fn busy_pane_pulses_a_dot_that_never_blinks_out() {
        // A busy pane always shows the dot (never None), and its colour changes
        // over the pulse — the trough (dim) differs from the peak (full).
        let trough = strip_marker(false, None, true, 0).expect("busy always shows a dot");
        let peak = strip_marker(false, None, true, PULSE_MS / 2).expect("busy always shows a dot");
        assert_eq!(trough.0, '●');
        assert_ne!(trough.1, peak.1, "the dot pulses between dim and bright");
        // Busy beats a plain activity dot; both are '●' but busy pulses.
        assert!(strip_marker(true, None, true, 0).is_some());
    }
}
