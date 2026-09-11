//! How a task row fits its width: where the bar sits, what a narrow block
//! gives up first, and that no two cells on a row ever overlap. The plan and
//! state tests live in `chatswarmrows_tests`; these are the width sweep.
use super::tests::{diamond, pane_with, row_text, state};
use super::*;
use crew_hive::TaskState;

#[test]
fn the_bar_sits_at_the_right_edge_and_tracks_the_run_clock() {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 0);
    state(&mut p, 10, TaskState::Done, 500);
    state(&mut p, 11, TaskState::Running, 500);
    let cells = cells(&p, 100, 0, 1000);
    let bar_w = crate::chatswarmspan::bar_cols(100) as usize;
    let last = 100 - 2; // one column of margin after the bar
    for (row, want) in [(0, "━━━━━━━━━━──────────"), (1, "──────────━━━━━━━━━━")]
    {
        let text = row_text(&cells, row);
        let bar: String = text
            .chars()
            .rev()
            .take(bar_w)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        assert_eq!(bar, want, "row {row}: {text}");
        let end = cells
            .iter()
            .filter(|c| c.row == row)
            .map(|c| c.col)
            .max()
            .unwrap();
        assert_eq!(end, last as u16);
    }
}

#[test]
fn width_sweep_keeps_every_row_inside_the_pane_with_the_bar_gone_when_narrow() {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 0);
    for cols in [30u16, 40, 60, 100, 160] {
        let cells = cells(&p, cols, 0, 900);
        for c in &cells {
            assert!(
                c.col + crate::chatwidth::char_w(c.c) as u16 <= cols,
                "cols={cols}: cell {:?} at col {} escapes the pane",
                c.c,
                c.col
            );
        }
        let r0 = row_text(&cells, 0);
        let has_bar = r0.contains('\u{2501}');
        assert_eq!(has_bar, cols >= 56, "cols={cols}: {r0}");
        assert!(
            r0.contains("research"),
            "cols={cols}: the title survives: {r0}"
        );
        let r2 = row_text(&cells, 2);
        assert!(r2.contains("review the draft"), "cols={cols}: {r2}");
        assert!(
            r2.contains("\u{2190} 1,2"),
            "cols={cols}: the deps stay: {r2}"
        );
        assert_eq!(
            r2.contains("critic"),
            cols >= 40,
            "cols={cols}: the specialist is the first column to go: {r2}"
        );
        let r1 = row_text(&cells, 1);
        assert_eq!(
            r1.contains("writer"),
            cols >= 40,
            "cols={cols}: and it goes on every row at once: {r1}"
        );
        assert_eq!(
            crate::chatswarmview::swarm_rows(&p, cols),
            4,
            "cols={cols}: the line plus three rows"
        );
    }
}

#[test]
fn under_pressure_the_specialist_goes_before_the_deps_and_the_title_clips_last() {
    let p = pane_with(diamond());
    let s = p.swarm.as_ref().unwrap();
    let full = (
        "critic  ".to_string(),
        "review the draft".to_string(),
        " \u{2190} 1,2".to_string(),
    );
    assert_eq!(words(s, 2, 80, 0), full);
    assert_eq!(level(s, 3, 80), 0);
    // 24 columns: the third row needs 30 whole, 22 without its specialist.
    assert_eq!(level(s, 3, 24), 1);
    let no_spec = (
        String::new(),
        "review the draft".to_string(),
        " \u{2190} 1,2".to_string(),
    );
    assert_eq!(words(s, 2, 24, 1), no_spec);
    // 18: the deps go too; 10: the title itself is cut and marked.
    assert_eq!(level(s, 3, 18), 2);
    assert_eq!(
        words(s, 2, 18, 2),
        (String::new(), "review the draft".into(), String::new())
    );
    assert_eq!(level(s, 3, 10), 2);
    assert_eq!(words(s, 2, 10, 2).1, "review th\u{2026}");
    // The record's line is always whole.
    assert_eq!(
        crate::chatswarmrec::plain(s, 2),
        " 3 \u{25cb} critic  review the draft \u{2190} 1,2"
    );
}

#[test]
fn cells_on_one_row_never_overlap_at_any_width() {
    let mut p = pane_with(diamond());
    state(&mut p, 10, TaskState::Running, 0);
    state(&mut p, 10, TaskState::Done, 5);
    state(&mut p, 11, TaskState::Running, 5);
    for cols in 0..=160u16 {
        let cells = cells(&p, cols, 0, 10);
        for row in 0..3u16 {
            let mut spans: Vec<(u16, u16)> = cells
                .iter()
                .filter(|c| c.row == row)
                .map(|c| (c.col, c.col + crate::chatwidth::char_w(c.c) as u16))
                .collect();
            spans.sort_unstable();
            for w in spans.windows(2) {
                assert!(
                    w[0].1 <= w[1].0,
                    "cols={cols} row={row}: {:?} vs {:?}",
                    w[0],
                    w[1]
                );
            }
        }
    }
}
