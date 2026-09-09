use super::*;
use crate::chattool::ToolDone;
use crate::chattoolline::{text_rows, TEXT_ROWS};

fn line(kind: LineKind, args: &str, result: Option<&str>) -> ToolLine {
    ToolLine {
        kind,
        label: "sys:run".into(),
        args: args.into(),
        args_short: String::new(),
        started_ms: 0,
        done: result.map(|t| ToolDone {
            ok: true,
            ms: 1,
            text: t.into(),
        }),
        show_text: true,
    }
}

#[test]
fn a_call_opens_as_its_arguments_then_the_result() {
    let l = line(
        LineKind::Tool,
        r#"{"cmd":"cargo  test\n--all","cwd":"/x","n":3}"#,
        Some("ok\nran 3"),
    );
    assert_eq!(
        opened_rows(&l),
        [
            "cmd: cargo test --all",
            "cwd: /x",
            "n: 3",
            "\u{2192} result",
            "ok",
            "ran 3"
        ]
    );
    let pending = line(LineKind::Tool, r#"{"cmd":"ls"}"#, None);
    assert_eq!(
        opened_rows(&pending),
        ["cmd: ls"],
        "args alone while it runs"
    );
    assert!(pending.has_text(), "so a pending line is clickable");
    let bare = line(LineKind::Tool, "{}", Some("out"));
    assert_eq!(opened_rows(&bare), ["out"], "no args, no separator");
    let raw = line(LineKind::Tool, "not json\nat all", None);
    assert_eq!(opened_rows(&raw), ["not json", "at all"]);
}

#[test]
fn long_arguments_and_long_results_are_capped_with_a_count() {
    let args: String = format!(
        "{{{}}}",
        (0..9)
            .map(|i| format!("\"k{i}\":\"v{i}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    let result: String = (1..=TEXT_ROWS + 3)
        .map(|i| format!("l{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let rows = opened_rows(&line(LineKind::Tool, &args, Some(&result)));
    assert_eq!(rows.len(), ARGS_ROWS + 1 + TEXT_ROWS);
    assert_eq!(rows[ARGS_ROWS - 1], "\u{2026} +4 more", "{rows:?}");
    assert_eq!(rows[ARGS_ROWS], "\u{2192} result");
    assert_eq!(rows[ARGS_ROWS + 1], "l1");
    assert_eq!(rows.last().unwrap(), "\u{2026} +4 lines");
}

#[test]
fn a_load_opens_as_its_detail_one_segment_per_row_tool_names_listed() {
    let l = line(LineKind::Mcp, "", Some("connected \u{b7} 3 tools: a, b, c"));
    assert_eq!(
        opened_rows(&l),
        ["connected", "3 tools:", "  a", "  b", "  c"]
    );
    let s = line(
        LineKind::Skill,
        "",
        Some("applied \u{b7} Check: unsafe, first."),
    );
    assert_eq!(
        opened_rows(&s),
        ["applied", "Check: unsafe, first."],
        "only a tool list is split"
    );
}

#[test]
fn text_rows_paint_exactly_the_opened_rows_and_only_when_open() {
    let _g = crate::app::motion_test_guard();
    let mut l = line(LineKind::Tool, r#"{"cmd":"ls"}"#, Some("a\nb"));
    let rows = text_rows(&l, 40);
    let texts: Vec<String> = rows
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| c.c)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    assert_eq!(
        texts,
        ["    cmd: ls", "    \u{2192} result", "    a", "    b"]
    );
    assert!(rows.iter().all(|r| r.len() == 40), "padded to the width");
    l.show_text = false;
    assert!(text_rows(&l, 40).is_empty());
}
