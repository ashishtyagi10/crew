//! How the native turns are cut: which results become stubs, what a stub
//! names, and when only the last round is kept.
use super::{shrink, size, stubbed};
use crate::provider::{ToolInvocation, ToolOutcome, Turn};

fn call(id: &str, input: serde_json::Value) -> ToolInvocation {
    ToolInvocation {
        id: id.into(),
        name: "sys__read_file".into(),
        input,
        bad_args: None,
    }
}

fn round(id: &str, input: serde_json::Value, said: &str, result: &str) -> [Turn; 2] {
    [
        Turn::Assistant {
            text: said.into(),
            calls: vec![call(id, input)],
        },
        Turn::ToolResults(vec![ToolOutcome {
            id: id.into(),
            name: "sys__read_file".into(),
            content: result.into(),
            is_error: false,
        }]),
    ]
}

fn label(_: &ToolInvocation) -> String {
    "sys:read_file".into()
}

fn contents(turns: &[Turn]) -> Vec<String> {
    let each = |t: &Turn| match t {
        Turn::ToolResults(rs) => rs.iter().map(|r| r.content.clone()).collect(),
        _ => vec![],
    };
    turns.iter().flat_map(each).collect()
}

/// A short result (an error) stays whole: its stub would be longer.
#[test]
fn a_stub_names_the_path_or_the_first_string_and_is_stable() {
    let long = "x".repeat(500);
    let mut turns: Vec<Turn> = round("1", serde_json::json!({"path": "src/a.rs"}), "", &long)
        .into_iter()
        .chain(round(
            "2",
            serde_json::json!({"pattern": "fn  main"}),
            "",
            &long,
        ))
        .chain(round("3", serde_json::json!({}), "", &long))
        .chain(round(
            "4",
            serde_json::json!({"path": "e.rs"}),
            "",
            "no such file",
        ))
        .chain(round("5", serde_json::json!({"path": "d.rs"}), "", &long))
        .collect();
    turns = stubbed(&turns, &label);
    let tail = "shortened to fit \u{2014} call it again if you need it]";
    assert_eq!(
        contents(&turns),
        [
            format!("[result of sys:read_file src/a.rs {tail}"),
            format!("[result of sys:read_file fn main {tail}"),
            format!("[result of sys:read_file {tail}"),
            "no such file".to_string(),
            long,
        ]
    );
    assert_eq!(stubbed(&turns, &label), turns, "a stub stubbed again");
}

/// Stubs when they come in under the largest request that went through;
/// otherwise the last round alone: an older round that carried a whole file
/// in its call (a write) is still too big with its result stubbed.
#[test]
fn when_the_stubs_do_not_come_under_what_fit_only_the_last_round_is_kept() {
    let read = |id: &str, path: &str| {
        round(
            id,
            serde_json::json!({ "path": path }),
            "",
            &"r".repeat(3_000),
        )
    };
    let reads: Vec<Turn> = read("1", "a.rs")
        .into_iter()
        .chain(read("2", "b.rs"))
        .collect();
    let mut cut = reads.clone();
    assert!(shrink(&mut cut, 100, 100 + size(0, &reads) - 1, &label));
    assert_eq!(cut.len(), 4, "stubs");
    assert!(contents(&cut)[0].starts_with("[result of sys:read_file a.rs"));

    let write = serde_json::json!({"path": "w.rs", "content": "x".repeat(9_000)});
    let turns: Vec<Turn> = round("1", write, "writing", "ok")
        .into_iter()
        .chain(read("2", "b.rs"))
        .collect();
    let mut last = turns.clone();
    assert!(shrink(&mut last, 100, 5_000, &label));
    assert_eq!(last, turns[2..], "the last round alone");
    let mut one = turns[2..].to_vec();
    assert!(!shrink(&mut one, 100, 5_000, &label), "nothing left to cut");
}
