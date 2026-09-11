use super::*;

fn tool(server: &str, name: &str) -> McpTool {
    McpTool {
        server: server.into(),
        name: name.into(),
        description: "does an unrelated thing".into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

/// `n` tools nobody asked for, none of them crew's own.
fn filler(n: usize) -> Vec<McpTool> {
    (0..n)
        .map(|i| tool("noise", &format!("thing{i}")))
        .collect()
}

fn doors(tools: &[McpTool]) -> usize {
    tools
        .iter()
        .filter(|t| t.server == "sys" && t.name == "find_tools")
        .count()
}

#[test]
fn over_the_budget_with_the_sys_surface_off_the_door_is_still_on_the_list() {
    let (kept, left_out) = select(filler(BUDGET + 10), "anything at all");
    assert_eq!(left_out, 10);
    assert_eq!(doors(&kept), 1, "sys:find_tools was added: {kept:?}");
    assert_eq!(kept.len(), BUDGET + 1, "the budget's worth, plus the door");
    assert!(
        !kept.iter().any(|t| t.server == "sys" && t.name == "run"),
        "the door is the ONLY sys tool the switch does not hide"
    );
}

#[test]
fn under_the_budget_with_the_sys_surface_off_nothing_is_added() {
    let (kept, left_out) = select(filler(5), "anything at all");
    assert_eq!(left_out, 0);
    assert_eq!(kept.len(), 5);
    assert_eq!(doors(&kept), 0, "no door when nothing is hidden: {kept:?}");
}

#[test]
fn with_the_sys_surface_on_the_door_is_not_listed_twice() {
    let mut tools = crate::broker::systools::tools();
    tools.extend(filler(BUDGET + 10));
    let (kept, _) = select(tools, "anything at all");
    assert_eq!(doors(&kept), 1, "{kept:?}");
}

#[test]
fn the_native_note_is_none_when_nothing_was_left_out() {
    assert_eq!(native_note(0), None);
}

#[test]
fn the_native_note_counts_what_was_left_out_and_names_the_wire_door() {
    let note = native_note(16).expect("something was left out");
    assert!(note.contains("16"), "{note}");
    assert!(note.contains("sys__find_tools"), "the WIRE name: {note}");
    assert!(
        !note.contains("sys:find_tools"),
        "not crew's spelling: {note}"
    );
    assert!(!note.contains('\n'), "one line: {note:?}");
}

#[test]
fn a_search_query_reads_q_and_tolerates_junk() {
    assert_eq!(search_query(r#"{"q": "calendar"}"#), "calendar");
    assert_eq!(search_query("not json"), "");
    assert_eq!(search_query(r#"{"query": "x"}"#), "");
}

#[test]
fn specs_keep_crew_spelling_and_the_schema() {
    let specs = specs_of(vec![tool("gcal", "events")]);
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].label(), "gcal:events");
    assert_eq!(specs[0].input_schema, serde_json::json!({"type": "object"}));
}
