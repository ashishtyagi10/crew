use super::*;

fn frame(json: &str) -> Vec<Frag> {
    frags(&serde_json::from_str(json).unwrap())
}

#[test]
fn fragments_of_one_call_assemble_with_their_arguments_concatenated() {
    let mut asm = CallAsm::default();
    for f in [
        r#"{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"fs_read","arguments":""}}]}"#,
        r#"{"tool_calls":[{"index":0,"function":{"arguments":"{\"pa"}}]}"#,
        r#"{"tool_calls":[{"index":0,"function":{"arguments":"th\":\"a.rs\"}"}}]}"#,
    ] {
        for frag in frame(f) {
            asm.push(frag);
        }
    }
    let calls = asm.finish();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].id, "call_1");
    assert_eq!(calls[0].name, "fs_read");
    assert_eq!(calls[0].input, serde_json::json!({"path": "a.rs"}));
}

#[test]
fn interleaved_calls_are_kept_apart_by_index() {
    let mut asm = CallAsm::default();
    for f in [
        r#"{"tool_calls":[{"index":0,"id":"a","function":{"name":"one","arguments":"{"}}]}"#,
        r#"{"tool_calls":[{"index":1,"id":"b","function":{"name":"two","arguments":"{}"}}]}"#,
        r#"{"tool_calls":[{"index":0,"function":{"arguments":"}"}}]}"#,
    ] {
        for frag in frame(f) {
            asm.push(frag);
        }
    }
    let calls = asm.finish();
    let names: Vec<&str> = calls.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["one", "two"]);
    assert_eq!(calls[0].input, serde_json::json!({}));
}

#[test]
fn a_frame_without_an_index_takes_its_array_position_and_a_nameless_call_is_dropped() {
    let fs = frame(
        r#"{"tool_calls":[{"function":{"arguments":"x"}},{"id":"q","function":{"name":"n","arguments":"bad json"}}]}"#,
    );
    assert_eq!(fs[0].index, 0);
    assert_eq!(fs[1].index, 1);
    let mut asm = CallAsm::default();
    fs.into_iter().for_each(|f| asm.push(f));
    let calls = asm.finish();
    assert_eq!(calls.len(), 1, "the nameless fragment never became a call");
    assert_eq!(
        calls[0].input,
        serde_json::json!({}),
        "bad JSON args become {{}}"
    );
}

#[test]
fn no_tool_calls_field_means_no_fragments() {
    assert!(frame(r#"{"content":"hi"}"#).is_empty());
}
