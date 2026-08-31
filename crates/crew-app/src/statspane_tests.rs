use super::*;

/// The section offsets `cells` walks down must land the LOG exactly where
/// [`crate::navlayout`] says it does — the draw and the hit paths read the
/// same numbers from two different code paths, and this is the seam.
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
    let panes = Vec::new();
    let (cols, rows) = (26u16, 48u16);
    let l = s.layout(rows, log.len(), 0);
    assert!(l.log_lines > 0, "the fixture has room for a LOG");
    let cells = s.cells(cols, rows, &panes, &log, 0);
    // The `LOG` legend sits on the rule row the layout named.
    let legend: String = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == l.log_top).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(legend.contains("LOG"), "row {}: {legend:?}", l.log_top);
    // The block's last row is its gap: the PANES rule below it needs air,
    // and a LOG that grew into the gap would sit flush against it.
    let gap_row = l.log_top + l.log_block() - 1;
    assert!(
        !cells.iter().any(|c| c.row == gap_row),
        "row {gap_row} is the LOG block's trailing gap and must stay empty"
    );
    // …and the PANES rule is on the very next row.
    assert_eq!(l.panes_top, gap_row + 1);
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
    let panes: Vec<PaneRow> = (0..n)
        .map(|i| PaneRow {
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
    let l = s.layout(rows, log.len(), n);
    let cells = s.cells(cols, rows, &panes, &log, 0);
    for (k, p) in panes.iter().enumerate() {
        // The row this pane's own TITLE was drawn on, found in the frame
        // rather than recomputed from the offsets under test. The title,
        // not the index: the mix draws its chips in the gutter too,
        // and a finder that cannot tell them apart proves nothing.
        let drawn = cells
            .iter()
            .filter(|c| c.row >= l.panes_top && c.col >= 5)
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
        let hit = crate::hit::sidebar_pane_index(drawn.row + 1, l.panes_top);
        assert_eq!(
            hit,
            Some(k),
            "git={git} log={log_len} panes={n} rows={rows}: \
             pane {k} drawn on content row {} maps to {hit:?}",
            drawn.row
        );
    }
}
