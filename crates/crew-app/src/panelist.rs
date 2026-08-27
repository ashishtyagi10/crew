//! Sidebar PANES section: a live list of open panes (index, name/title, a `▸`
//! focus marker, and an activity dot) so the whole grid is visible at a glance —
//! handy when a single pane is zoomed. A one-row **pulse** chart under the
//! header traces how many panes were busy each second — the crew's workload
//! as a moving line — and busy rows carry a live spinner in the accent color.
use crew_render::CellView;

use crate::boxdraw::section_header;

use crate::palette::accent;

/// One row of the PANES list.
pub struct PaneRow {
    pub index: usize,
    pub title: String,
    pub focused: bool,
    pub activity: bool,
    /// Not visible in the content area — minimized into the nav (the pane's
    /// `[-]` border button) or covered while another pane is zoomed: drawn with
    /// a right-aligned `[+]`; clicking the row focuses the pane, which brings
    /// it back on screen.
    pub minimized: bool,
    /// A raised "needs you" marker: `(glyph, visible)`. The glyph names the
    /// event (`!` bell · `⚑` pattern · `✓` command done); `visible` is the
    /// blink phase (false hides the marker mid-pulse, the tint stays). Drawn
    /// in the bell colour, superseding the quiet activity dot.
    pub attention: Option<(char, bool)>,
    /// Doing background work (swarm running, agent chat awaiting, Far op):
    /// the row's dot slot spins while this holds. Attention still wins.
    pub busy: bool,
    /// Lines that arrived since this pane was last read
    /// ([`crate::unread`]) — the same count the pane's own card and its
    /// minimized thumbnail carry. `0` draws nothing.
    pub unread: usize,
    /// The pointer is on this row. The whole row is a click target that
    /// focuses (and restores) the pane, and nothing said so: it lifts its ink
    /// out of the muted grey rather than washing a background behind it,
    /// because the page's contrast headroom is already spent.
    pub hovered: bool,
}

/// Render the PANES section: a `PANES` rule on row 0, the crew-pulse chart on
/// row 1 (always reserved, so the click → pane-row mapping in `hit.rs` stays
/// static), then one row per pane (up to `limit`) beneath it.
pub fn pane_cells(
    panes: &[PaneRow],
    cols: u16,
    limit: usize,
    pulse: &crate::spark::History,
    spin: char,
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = section_header("PANES", cols, t.border_normal, accent(), t.page_bg);
    if cols > 5 {
        // Auto-scaled to its own peak: one busy pane still draws a full-height
        // blip, a swarm of six reads as a mountain range.
        out.extend(crate::spark::line_cells(
            pulse,
            cols.saturating_sub(4),
            3,
            1,
            0,
            accent(),
        ));
    }
    for (k, p) in panes.iter().take(limit).enumerate() {
        let row = 2 + k as u16;
        let head = format!("{} {}", if p.focused { '▸' } else { ' ' }, p.index);
        let head_fg = if p.focused || p.hovered {
            accent()
        } else {
            t.text_muted
        };
        write(&mut out, &head, 2, row, head_fg, cols - 1, t.page_bg);
        let tstart = 2 + head.chars().count() as u16 + 1;
        let title_fg = if p.focused || p.hovered {
            t.ink
        } else if p.attention.is_some() {
            t.bell
        } else {
            t.text_muted
        };
        // The count sits between the title and the dot slot: the third place
        // this number appears (card, thumbnail, here), because the sidebar is
        // the one view that lists panes you cannot see.
        let count = crate::unread::badge(p.unread).filter(|_| !p.focused);
        let cw = count.as_ref().map_or(0, |n| n.chars().count() as u16 + 1);
        // A minimized row carries a right-aligned [+] restore button (ending a
        // cell left of the activity-dot slot); its title stops short of it.
        let tmax = if p.minimized {
            cols.saturating_sub(8 + cw)
        } else {
            cols.saturating_sub(3 + cw)
        };
        write(&mut out, &p.title, tstart, row, title_fg, tmax, t.page_bg);
        if p.minimized {
            write(
                &mut out,
                "[+]",
                cols.saturating_sub(6),
                row,
                accent(),
                cols.saturating_sub(2),
                t.page_bg,
            );
        }
        if let Some(n) = count {
            let w = n.chars().count() as u16;
            write(
                &mut out,
                &n,
                cols.saturating_sub(3 + w),
                row,
                t.activity,
                cols.saturating_sub(2),
                t.page_bg,
            );
        }
        // The attention marker owns the dot slot while raised; the quiet
        // activity dot returns once the pane has been looked at.
        if let Some((glyph, on)) = p.attention {
            if on {
                write(
                    &mut out,
                    &glyph.to_string(),
                    cols.saturating_sub(2),
                    row,
                    t.bell,
                    cols,
                    t.page_bg,
                );
            }
        } else if p.busy {
            // Live spinner: the busy pane repaints continuously anyway, so the
            // sidebar frame is free — the row visibly *works*.
            write(
                &mut out,
                &spin.to_string(),
                cols.saturating_sub(2),
                row,
                accent(),
                cols,
                t.page_bg,
            );
        } else if p.activity {
            write(
                &mut out,
                "●",
                cols.saturating_sub(2),
                row,
                t.activity,
                cols,
                t.page_bg,
            );
        }
    }
    out
}

/// Write `s` at `(col, row)`, stopping before `max_col`.
fn write(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    max_col: u16,
    bg: (u8, u8, u8),
) {
    // Width-aware: pane titles can carry emoji/CJK (OSC titles) — a wide
    // glyph advances two columns (see `chatwidth`).
    crate::chatwidth::place_row(col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(index: usize, title: &str, focused: bool, activity: bool) -> PaneRow {
        PaneRow {
            index,
            title: title.into(),
            focused,
            activity,
            minimized: false,
            attention: None,
            busy: false,
            hovered: false,
            unread: 0,
        }
    }

    /// The count appears in the sidebar too — the one view that lists panes
    /// you cannot see — and never on the row you are looking at.
    #[test]
    fn an_unread_count_rides_the_row_of_a_pane_you_are_not_in() {
        let _g = crate::app::theme_test_guard();
        let quiet = row(1, "sh", false, false);
        let loud = PaneRow {
            unread: 12,
            ..row(2, "sh", false, false)
        };
        let focused = PaneRow {
            unread: 12,
            ..row(3, "sh", true, false)
        };
        let text = |p: &PaneRow| -> String {
            let cells = cells_of(std::slice::from_ref(p), 30, 4);
            let mut v: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == 2).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert!(text(&loud).contains("12"), "{:?}", text(&loud));
        assert!(!text(&quiet).contains("12"));
        assert!(
            !text(&focused).contains("12"),
            "the pane you are in cannot have unread lines"
        );
    }

    /// A long title gives way to the count rather than overprinting it.
    #[test]
    fn the_title_stops_short_of_the_count() {
        let _g = crate::app::theme_test_guard();
        let long = PaneRow {
            unread: 7,
            ..row(1, "a-very-long-pane-title-indeed", false, false)
        };
        let cells = cells_of(std::slice::from_ref(&long), 30, 4);
        let at = |col: u16| cells.iter().filter(|c| c.row == 2 && c.col == col).count();
        assert!(at(26) <= 1, "two glyphs share a cell on the count's row");
        let digit = cells
            .iter()
            .find(|c| c.row == 2 && c.c == '7')
            .expect("the count was pushed off the row");
        assert!(digit.col >= 26, "the count moved out of its slot");
    }

    /// `pane_cells` with an empty pulse history and a fixed spinner glyph.
    fn cells_of(panes: &[PaneRow], cols: u16, limit: usize) -> Vec<crew_render::CellView> {
        pane_cells(panes, cols, limit, &crate::spark::History::new(8), '⠋')
    }

    /// The row under the pointer must look different from the quiet rows
    /// around it — the whole row focuses (and restores) its pane on a click,
    /// and until now nothing on screen said so.
    #[test]
    fn a_hovered_row_lifts_its_ink_out_of_the_muted_grey() {
        let quiet = [row(1, "build", false, false)];
        let hot = [PaneRow {
            hovered: true,
            ..row(1, "build", false, false)
        }];
        let ink_of = |rows: &[PaneRow]| -> Vec<(u8, u8, u8)> {
            cells_of(rows, 24, 10)
                .iter()
                .filter(|c| c.row == 2)
                .map(|c| c.fg)
                .collect()
        };
        let (a, b) = (ink_of(&quiet), ink_of(&hot));
        assert_eq!(a.len(), b.len(), "hover must not change what is drawn");
        assert_ne!(a, b, "hover must change how it is drawn");
        // Specifically: up to the theme's full-contrast ink, never a wash.
        let t = crew_theme::theme();
        assert!(b.contains(&t.ink), "hovered title reaches the ink");
        assert!(!a.contains(&t.ink), "a quiet title does not");
    }

    #[test]
    fn pane_cells_marks_minimized_panes_with_a_restore_button() {
        let panes = [
            row(1, "build", true, false),
            PaneRow {
                minimized: true,
                ..row(2, "server", false, false)
            },
        ];
        let cells = cells_of(&panes, 24, 10);
        // The minimized pane's row carries a right-aligned [+] restore button
        // ending one cell left of the activity-dot slot: cols 18..=20. Pane
        // rows start at row 2 — row 1 belongs to the pulse chart.
        let at = |col: u16, row: u16| {
            cells
                .iter()
                .find(|c| c.row == row && c.col == col)
                .map(|c| c.c)
        };
        assert_eq!(at(18, 3), Some('['));
        assert_eq!(at(19, 3), Some('+'));
        assert_eq!(at(20, 3), Some(']'));
        // …and only on minimized rows.
        assert!(!cells.iter().any(|c| c.c == '+' && c.row == 2));
    }

    #[test]
    fn pane_cells_lists_focus_and_activity() {
        let panes = [row(1, "build", true, false), row(2, "server", false, true)];
        let cells = cells_of(&panes, 24, 10);
        // PANES rule on row 0
        assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
        assert!(cells.iter().any(|c| c.c == 'P' && c.row == 0));
        // focus marker + title for the focused pane on row 2 (row 1 = pulse)
        assert!(cells.iter().any(|c| c.c == '▸' && c.row == 2));
        assert!(cells
            .iter()
            .any(|c| c.c == 'b' && c.row == 2 && c.fg == crew_theme::theme().ink));
        // the unfocused pane's title is dimmed on row 3, with an activity dot
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 3 && c.fg == crew_theme::theme().text_muted));
        assert!(cells
            .iter()
            .any(|c| c.c == '●' && c.row == 3 && c.fg == crew_theme::theme().activity));
    }

    #[test]
    fn busy_row_spins_in_the_accent_color_and_attention_still_wins() {
        let mut busy = row(1, "swarm", false, true);
        busy.busy = true;
        let cells = cells_of(&[busy], 24, 10);
        // The spinner glyph owns the dot slot, accent-colored; the quiet
        // activity dot yields to it.
        assert!(cells
            .iter()
            .any(|c| c.c == '⠋' && c.row == 2 && c.col == 22 && c.fg == accent()));
        assert!(!cells.iter().any(|c| c.c == '●' && c.row == 2));
        // Attention beats the spinner: the needs-you marker is the loudest.
        let mut both = row(1, "swarm", false, false);
        both.busy = true;
        both.attention = Some(('!', true));
        let cells = cells_of(&[both], 24, 10);
        assert!(cells
            .iter()
            .any(|c| c.c == '!' && c.row == 2 && c.col == 22));
        assert!(!cells.iter().any(|c| c.c == '⠋'));
    }

    #[test]
    fn pulse_chart_traces_history_under_the_header() {
        let mut h = crate::spark::History::new(8);
        for v in [0, 2, 4] {
            h.push(v);
        }
        let cells = pane_cells(&[row(1, "x", false, false)], 24, 10, &h, '⠋');
        // Three samples land right-aligned on row 1; the newest (peak) column
        // draws the tallest block.
        let chart: Vec<_> = cells.iter().filter(|c| c.row == 1).collect();
        assert_eq!(chart.len(), 3);
        assert_eq!(chart.iter().map(|c| c.c).max(), Some('█'));
    }

    #[test]
    fn attention_row_draws_the_marker_and_tints_the_title() {
        let panes = [
            row(1, "build", true, false),
            PaneRow {
                attention: Some(('!', true)),
                ..row(2, "server", false, true)
            },
        ];
        let cells = cells_of(&panes, 24, 10);
        let bell = crew_theme::theme().bell;
        // marker glyph in the dot slot, in the bell (needs-you) colour
        assert!(cells
            .iter()
            .any(|c| c.c == '!' && c.row == 3 && c.col == 22 && c.fg == bell));
        // the title is tinted too, so the row is findable at a glance
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 3 && c.fg == bell));
        // attention supersedes the quiet activity dot
        assert!(!cells.iter().any(|c| c.c == '●' && c.row == 3));
    }

    #[test]
    fn attention_blink_off_phase_hides_the_marker_but_keeps_the_tint() {
        let panes = [PaneRow {
            attention: Some(('!', false)),
            ..row(1, "server", false, false)
        }];
        let cells = cells_of(&panes, 24, 10);
        let bell = crew_theme::theme().bell;
        assert!(!cells.iter().any(|c| c.c == '!' && c.row == 2));
        assert!(cells
            .iter()
            .any(|c| c.c == 's' && c.row == 2 && c.fg == bell));
    }

    #[test]
    fn pane_cells_respects_limit() {
        let panes: Vec<PaneRow> = (1..=5).map(|i| row(i, "x", false, false)).collect();
        let cells = cells_of(&panes, 24, 2);
        // only two pane rows (2 and 3) are drawn; nothing reaches row 4
        assert!(!cells.iter().any(|c| c.row == 4));
    }
}
