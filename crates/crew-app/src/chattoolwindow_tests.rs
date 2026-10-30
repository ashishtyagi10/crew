//! A live block folds its earlier calls under one row and keeps the last
//! three in view; the row counts what it folds and opens on a click.
use crate::chattool::{ToolBlock, ToolDone, ToolLine};
use crate::chattoolview::{block_lines, hit, ToolHit};

fn call(n: usize, done: Option<bool>) -> ToolLine {
    ToolLine {
        kind: crate::chattoolkind::LineKind::Tool,
        label: "Read".into(),
        args: String::new(),
        args_short: format!("f{n}.rs"),
        started_ms: 0,
        done: done.map(|ok| ToolDone {
            ok,
            ms: 5,
            text: String::new(),
        }),
        show_text: false,
    }
}

fn live(n: usize) -> ToolBlock {
    let mut lines: Vec<ToolLine> = (0..n).map(|i| call(i, Some(true))).collect();
    lines[1].done = Some(ToolDone {
        ok: false,
        ms: 5,
        text: String::new(),
    });
    lines[n - 1].done = None; // the one running now
    ToolBlock {
        agent_id: 1,
        agent: "claude".into(),
        lines,
        anchor: None,
        settled: false,
        expanded: false,
    }
}

fn rows(b: &ToolBlock) -> Vec<String> {
    block_lines(b, 0, 80, false)
        .iter()
        .map(|l| l.iter().map(|c| c.c).collect())
        .collect()
}

#[test]
fn twelve_live_calls_draw_a_head_and_the_last_three() {
    let b = live(12);
    let r = rows(&b);
    assert_eq!(r.len(), 4, "{r:#?}");
    assert!(r[0].contains("9 earlier"), "{}", r[0]);
    assert!(r[0].contains("1 failed"), "the folded failure: {}", r[0]);
    assert!(r[3].contains("f11.rs"), "the call running now: {}", r[3]);
    assert_eq!(hit(&b, 80, 0), Some(ToolHit::Summary), "the head opens it");
    assert_eq!(hit(&b, 80, 1), Some(ToolHit::Line(9)));
}

#[test]
fn an_opened_live_block_shows_every_call_under_its_head() {
    let mut b = live(12);
    b.expanded = true;
    let r = rows(&b);
    assert_eq!(r.len(), 13);
    assert!(r[0].contains("all 12 calls"), "{}", r[0]);
    assert_eq!(hit(&b, 80, 1), Some(ToolHit::Line(0)));
}

#[test]
fn a_short_block_and_a_settled_one_are_as_they_were() {
    assert_eq!(rows(&live(4)).len(), 4, "four calls, no head");
    assert_eq!(hit(&live(4), 80, 0), Some(ToolHit::Line(0)));
    let mut done = live(12);
    done.settled = true;
    assert_eq!(rows(&done).len(), 1, "the summary line alone");
}
