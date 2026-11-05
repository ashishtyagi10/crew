use super::*;
use crate::chattool::{ToolDone, ToolLine};

fn call(label: &str, ms: u64) -> ToolLine {
    ToolLine {
        kind: LineKind::Tool,
        label: label.into(),
        args: String::new(),
        args_short: String::new(),
        started_ms: 0,
        done: Some(ToolDone {
            ok: true,
            ms,
            text: String::new(),
        }),
        show_text: false,
    }
}

fn block(lines: Vec<ToolLine>) -> ToolBlock {
    ToolBlock {
        agent_id: 1,
        agent: "smith".into(),
        lines,
        anchor: None,
        settled: true,
        expanded: false,
    }
}

/// Six reads, three edits, two runs — Claude Code's names and crew's mixed,
/// as a relay turn mixes them.
fn turn() -> ToolBlock {
    let mut l: Vec<ToolLine> = (0..5).map(|_| call("Read", 200)).collect();
    l.push(call("sys:grep", 0));
    l.extend((0..3).map(|_| call("Edit", 1_000)));
    l.push(call("Bash", 4_000));
    l.push(call("sys:run", 4_000));
    block(l)
}

#[test]
fn the_summary_names_what_the_calls_were_in_the_order_they_ran() {
    assert_eq!(
        kinds_text(&turn()).unwrap(),
        "6 read \u{b7} 3 edit \u{b7} 2 run"
    );
    assert_eq!(
        crate::chattoolkind::summary_text(&turn()),
        "6 read \u{b7} 3 edit \u{b7} 2 run \u{b7} 12.0s"
    );
    assert_eq!(kinds_text(&block(Vec::new())), None);
}

#[test]
fn the_strip_gives_each_kind_its_share_of_the_time() {
    // 1 s reading, 3 s editing, 8 s running, over 12 cells.
    assert_eq!(
        widths(&turn(), 12),
        vec![("read", 1), ("edit", 3), ("run", 8)]
    );
    // Shares that do not divide: the cells still add to the width, and the
    // largest remainder takes the spare one.
    let odd = block(vec![call("Read", 1), call("Edit", 1), call("Bash", 1)]);
    assert_eq!(widths(&odd, 10), vec![("read", 4), ("edit", 3), ("run", 3)]);
    assert!(widths(&block(vec![call("Read", 0)]), 12).is_empty());
}

fn tail_cells(row: &CardLine) -> Vec<&crate::chatbody::CardCell> {
    row.iter().filter(|c| c.c == '\u{2501}').collect()
}

#[test]
fn the_strip_sits_on_the_summary_row_in_each_kinds_ink_or_not_at_all() {
    let _g = crate::app::theme_test_guard();
    let b = turn();
    let row = crate::chattoolview::summary(&b, 80, false);
    let cells = tail_cells(&row);
    assert_eq!(cells.len(), STRIP_CELLS);
    let added = crate::chatink::token_fg(Token::Added);
    let edit: Vec<_> = cells.iter().skip(1).take(3).collect();
    assert!(edit.iter().all(|c| c.fg == added), "edits in the added ink");
    assert_ne!(cells[0].fg, added, "a read is not an edit");
    // Too narrow for text and strip: the text stays, the strip goes.
    let text_w = 2 + 2 + crate::chattoolkind::summary_text(&b).chars().count();
    let narrow = crate::chattoolview::summary(&b, text_w + 3, false);
    assert!(tail_cells(&narrow).is_empty(), "never a second row");
}
