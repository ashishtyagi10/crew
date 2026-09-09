use super::*;
use crate::chatmsgs::card_lines;
use crate::chatmsgs::tests::msg;
use crate::chattool::{ToolDone, ToolLine};

fn done_line(ok: bool, ms: u64, text: &str) -> ToolLine {
    ToolLine {
        kind: crate::chattoolkind::LineKind::Tool,
        label: "fs:read".into(),
        args: String::new(),
        args_short: "a".into(),
        started_ms: 0,
        done: Some(ToolDone {
            ok,
            ms,
            text: text.into(),
        }),
        show_text: false,
    }
}

fn block(agent: &str, lines: Vec<ToolLine>) -> ToolBlock {
    ToolBlock {
        agent_id: 1,
        agent: agent.into(),
        lines,
        anchor: None,
        settled: false,
        expanded: false,
    }
}

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn texts(lines: &[CardLine]) -> Vec<String> {
    lines.iter().map(text).collect()
}

#[test]
fn the_summary_counts_calls_time_and_failures() {
    let _g = crate::app::motion_test_guard();
    let mut b = block(
        "coder",
        vec![done_line(true, 2_000, ""), done_line(false, 100, "")],
    );
    b.settled = true;
    assert_eq!(
        text(&summary(&b, 60, false)),
        "  \u{25b8} 2 tool calls \u{00b7} 2.1 s \u{00b7} 1 failed"
    );
    b.expanded = true;
    b.lines.pop();
    assert_eq!(
        text(&summary(&b, 60, false)),
        "  \u{25be} 1 tool call \u{00b7} 2.0 s"
    );
    assert!(
        text(&summary(&b, 60, true)).contains('\u{f0ad}'),
        "a wrench on the icon set"
    );
}

#[test]
fn block_rows_follow_live_then_collapsed_then_opened_and_hit_mirrors_them() {
    let _g = crate::app::motion_test_guard();
    let mut b = block(
        "coder",
        vec![done_line(true, 1, "x"), done_line(true, 1, "y\nz")],
    );
    assert_eq!(block_lines(&b, 0, 60, false).len(), 2, "live: every line");
    assert_eq!(hit(&b, 60, 1), Some(ToolHit::Line(1)));
    b.settled = true;
    assert_eq!(
        block_lines(&b, 0, 60, false).len(),
        1,
        "settled: the summary"
    );
    assert_eq!(hit(&b, 60, 0), Some(ToolHit::Summary));
    assert_eq!(hit(&b, 60, 1), None, "nothing under a collapsed block");
    b.expanded = true;
    b.lines[1].show_text = true;
    let rows = block_lines(&b, 0, 60, false);
    assert_eq!(rows.len(), 1 + 2 + 2, "summary, two lines, two text rows");
    assert_eq!(hit(&b, 60, 2), Some(ToolHit::Line(1)));
    assert_eq!(hit(&b, 60, 4), Some(ToolHit::Line(1)), "its text rows too");
    assert_eq!(hit(&b, 60, 5), None);
}

#[test]
fn a_settled_block_sits_above_its_reply_a_live_one_under_the_streaming_card() {
    let _g = crate::app::motion_test_guard();
    let mut settled = block("coder", vec![done_line(true, 1, "")]);
    settled.settled = true;
    settled.anchor = Some("77".into());
    let live = block("planner", vec![done_line(true, 1, "")]);
    let blocks = [settled, live];
    let mut reply = msg("coder \u{2192} user", "done");
    reply.ts = "77".into();
    let stream = msg("planner", "typing");
    let view = View {
        streaming_from: 1,
        tools: &blocks,
        ..View::default()
    };
    let rows = texts(&card_lines(&[&reply, &stream], 60, 0, view));
    assert!(rows[0].contains("1 tool call"), "summary first: {rows:?}");
    assert!(
        rows[1].contains("coder"),
        "then the reply's header: {rows:?}"
    );
    let stream_hdr = rows.iter().position(|r| r.contains("planner")).unwrap();
    assert!(
        rows[stream_hdr + 2].contains("\u{2713} fs:read a"),
        "the live line under the streaming body: {rows:?}"
    );
    assert!(orphan_ids(view, &[&reply, &stream]).is_empty());
}

#[test]
fn a_block_with_no_card_stands_as_its_own_thin_card_after_the_transcript() {
    let _g = crate::app::motion_test_guard();
    let blocks = [block("coder", vec![done_line(true, 1, "")])];
    let hello = msg("user", "hi");
    let view = View {
        tools: &blocks,
        ..View::default()
    };
    assert_eq!(orphan_ids(view, &[&hello]), [0]);
    let rows = texts(&card_lines(&[&hello], 60, 0, view));
    let n = rows.len();
    assert!(rows[n - 1].contains("\u{2713} fs:read a"), "{rows:?}");
    assert!(
        rows[n - 2].contains("coder"),
        "headed by the agent: {rows:?}"
    );
    assert_eq!(rows[n - 3], "", "after a card gap");
    let alone = texts(&card_lines(&[], 60, 0, view));
    assert_eq!(alone.len(), 2, "no gap when nothing precedes");
}
