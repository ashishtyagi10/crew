use super::*;
use crate::navlayout::Tail;

/// The Activity section must draw its LOG exactly where its layout says it does — the draw
/// and the hit paths (the LOG's wheel scroll) read the same numbers, and this is the seam.
#[test]
fn the_drawn_log_starts_on_the_row_the_layout_reserved() {
    let _g = crate::app::theme_test_guard();
    let mut s = StatsPane::new();
    s.git.set_info(Some(git::GitInfo {
        branch: "main".into(),
        changed: 0,
        ahead: 0,
        behind: 0,
    }));
    let log: Vec<crate::applog::LogEntry> = (0..6)
        .map(|i| crate::applog::LogEntry {
            level: crate::applog::LogLevel::Info,
            text: format!("12:00 line{i}"),
        })
        .collect();
    let panes: Vec<crate::panelist::PaneRow> = Vec::new();
    let (cols, rows) = (26u16, 48u16);
    let activity = crate::navsection::NavSection::Activity;
    let tail = crate::navlayout::Tail::Log(log.len());
    let l = crate::navsectionlayout::layout_for(activity, rows, tail, log.len(), 0);
    assert!(l.log_lines > 0, "the fixture has room for a LOG");
    let list = crate::tabnav::NavList::from(panes.clone());
    let input = crate::navpanel::PanelInput {
        panes: &list,
        log: &log,
        log_back: 0,
        glance: None,
        strip: None,
    };
    let cells = s.panel_cells(activity, &l, cols, rows, &input);
    // The `LOG` legend sits on the rule row the layout named.
    let legend: String = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == l.log_top).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(legend.contains("LOG"), "row {}: {legend:?}", l.log_top);
    // Every line the LOG drew is inside the block the layout gave it, and the block fits.
    assert!(cells.iter().all(|c| c.row < l.log_top + l.log_block()));
    assert!(l.log_top + l.log_block() <= rows);
    assert_eq!(l.panes_top, None, "Activity shows no pane list");
}

/// The seam [`crate::navlayout`] exists for, asserted end to end: for
/// every nav height, log depth and crew size, the row a pane's number is
/// DRAWN on is the row `hit::sidebar_pane_index` maps back to that pane.
///
/// The two used to be independent `+` chains, and the arithmetic test
/// beside the hit function only ever checked the chain against itself.
#[test]
fn a_click_lands_on_the_pane_row_the_frame_drew() {
    let _g = crate::app::theme_test_guard();
    for git in [false, true] {
        for log_len in [0usize, 1, 4, 40] {
            for n in [1usize, 3, 7] {
                for rows in [20u16, 34, 48, 70] {
                    check_seam(git, log_len, n, rows);
                }
            }
        }
    }
}

fn check_seam(git: bool, log_len: usize, n: usize, rows: u16) {
    let mut s = StatsPane::new();
    if git {
        s.set_git(Some(git::GitInfo {
            branch: "main".into(),
            changed: 0,
            ahead: 0,
            behind: 0,
        }));
    }
    let log: Vec<crate::applog::LogEntry> = (0..log_len)
        .map(|i| crate::applog::LogEntry {
            level: crate::applog::LogLevel::Info,
            text: format!("12:00 line{i}"),
        })
        .collect();
    let panes: Vec<crate::panelist::PaneRow> = (0..n)
        .map(|i| crate::panelist::PaneRow {
            index: i + 1,
            title: format!("{}pane", (b'a' + i as u8) as char),
            focused: false,
            activity: false,
            minimized: false,
            attention: None,
            busy: false,
            unread: 0,
            hovered: false,
        })
        .collect();
    let cols = 26u16;
    let sec = crate::navsection::NavSection::Panes;
    let l = crate::navsectionlayout::layout_for(sec, rows, Tail::Log(log.len()), log.len(), n);
    let list = crate::tabnav::NavList::from(panes.clone());
    let input = crate::navpanel::PanelInput {
        panes: &list,
        log: &log,
        log_back: 0,
        glance: None,
        strip: None,
    };
    let cells = s.panel_cells(sec, &l, cols, rows, &input);
    let top = l.panes_top.expect("the Panes section shows the list");
    for (k, p) in panes.iter().enumerate() {
        // The row this pane's own TITLE was drawn on, found in the frame
        // rather than recomputed from the offsets under test. The title,
        // not the index: the mix draws its chips in the gutter too,
        // and a finder that cannot tell them apart proves nothing.
        let drawn = cells
            .iter()
            .filter(|c| c.row >= top && c.col >= 5)
            .find(|c| {
                // The whole title, in consecutive columns: single glyphs
                // collide with the crew legend written on the same rows
                // ("waiting" has an `a` in it), and a finder that cannot
                // tell the two apart proves nothing.
                p.title.chars().enumerate().all(|(i, ch)| {
                    cells
                        .iter()
                        .any(|d| d.row == c.row && d.col == c.col + i as u16 && d.c == ch)
                })
            });
        let Some(drawn) = drawn else {
            continue; // this nav had no room for row k; nothing to click
        };
        // `rel_row` is measured from the card's OUTER top edge: +1 border.
        let hit = crate::hit::sidebar_pane_index(drawn.row + 1, top);
        assert_eq!(
            hit,
            Some(k),
            "git={git} log={log_len} panes={n} rows={rows}: \
             pane {k} drawn on content row {} maps to {hit:?}",
            drawn.row
        );
    }
}

/// Before the first CPU sample the chart is still a chart: its dotted
/// baseline, not a two-row gap under the dials.
#[test]
fn an_empty_cpu_history_draws_a_dotted_baseline() {
    let _g = crate::app::theme_test_guard();
    let s = StatsPane::new();
    assert!(s.cpu_hist.is_empty(), "a fresh pane has no history");
    let paint = s.system_paint(28, 60, 2.0);
    let row0 = f32::from(clock::CLOCK_H + CHART_OFF);
    let base: Vec<_> = paint
        .iter()
        .filter(|p| p.y >= row0 && p.y <= row0 + f32::from(CHART_ROWS))
        .collect();
    assert!(base.len() > 4, "a row of dots: {}", base.len());
    // Dots, not a solid rule: every one short of the width.
    assert!(
        base.iter().all(|p| p.w < 1.0),
        "dotted: {:?}",
        base.iter().map(|p| p.w).collect::<Vec<_>>()
    );
}
