use super::*;
use crate::provider::ToolOutcome;

/// Native turns read as the text loop's exchanges: every call of a batch
/// under the message that asked for them, each result whole.
#[test]
fn a_transcript_writes_each_call_with_its_result() {
    let call = |id: &str| ToolInvocation {
        id: id.into(),
        name: format!("t_{id}"),
        input: serde_json::json!({}),
    };
    let done = |id: &str, content: &str| ToolOutcome {
        id: id.into(),
        name: String::new(),
        content: content.into(),
        is_error: false,
    };
    let turns = [
        Turn::Assistant {
            text: "two reads".into(),
            calls: vec![call("a"), call("b")],
        },
        Turn::ToolResults(vec![done("a", "A".repeat(700).as_str()), done("b", "B")]),
    ];
    let t = transcript(&turns, |c| c.name.replace('_', ":"));
    let want = format!(
        "YOUR MESSAGE:\ntwo reads\nCALLED t:a {{}}\nRESULT:\n{}\n\nCALLED t:b {{}}\nRESULT:\nB",
        "A".repeat(700)
    );
    assert_eq!(t, want);
    let p = prompt("task", "", &refused("", [("t:c".into(), "{}".into())]));
    let want = "task\n\nTOOL EXCHANGES SO FAR:\nCALLED t:c {}\nRESULT:\nnot run \u{2014} tool budget spent";
    assert_eq!(p, format!("{want}\n\n{INSTRUCTION}"));
}
