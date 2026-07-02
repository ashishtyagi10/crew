use super::*;

fn no_stats() -> std::collections::HashMap<String, (u32, u64)> {
    std::collections::HashMap::new()
}

fn agent(name: &str, model: &str) -> AgentInfo {
    AgentInfo {
        name: name.into(),
        role: String::new(),
        model: model.into(),
    }
}

fn text(cells: &[CellView]) -> String {
    let mut v: Vec<(u16, char)> = cells.iter().map(|c| (c.col, c.c)).collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn short_model_strips_provider_and_variant() {
    assert_eq!(
        short_model("meta-llama/llama-3.3-70b-instruct:free"),
        "llama-3.3-70b-instruct"
    );
    assert_eq!(short_model("claude-sonnet-5"), "claude-sonnet-5");
    assert_eq!(short_model(""), "");
}

#[test]
fn agent_color_is_stable_and_distinguishes_names() {
    assert_eq!(agent_color("planner"), agent_color("planner"));
}

#[test]
fn roster_row_lists_agents_with_model_badges() {
    let agents = [agent("planner", "org/m-1:free"), agent("coder", "")];
    let line = text(&roster_cells(80, 1, &agents, &[], &no_stats()));
    assert!(line.contains("planner m-1"), "got: {line}");
    assert!(line.contains("coder"), "got: {line}");
}

#[test]
fn active_agent_gets_arrow_marker_and_bold_name() {
    let agents = [agent("planner", ""), agent("coder", "")];
    let cells = roster_cells(80, 1, &agents, &["coder"], &no_stats());
    let line = text(&cells);
    assert!(line.contains("\u{25b8} coder"), "got: {line}");
    assert!(line.contains("\u{25aa} planner"), "got: {line}");
    assert!(cells.iter().any(|c| c.bold), "active chip should be bold");
}

#[test]
fn several_active_agents_highlight_together() {
    let agents = [
        agent("planner", ""),
        agent("coder", ""),
        agent("reviewer", ""),
    ];
    let line = text(&roster_cells(
        80,
        1,
        &agents,
        &["planner", "coder"],
        &no_stats(),
    ));
    assert!(line.contains("\u{25b8} planner"), "got: {line}");
    assert!(line.contains("\u{25b8} coder"), "got: {line}");
    assert!(line.contains("\u{25aa} reviewer"), "got: {line}");
}

#[test]
fn roster_clips_to_width_and_targets_row() {
    let agents = [agent("a-very-long-agent-name", "some/very-long-model")];
    let cells = roster_cells(10, 1, &agents, &[], &no_stats());
    assert!(cells.iter().all(|c| c.col < 10 && c.row == 1));
}

#[test]
fn empty_roster_renders_nothing() {
    assert!(roster_cells(80, 1, &[], &[], &no_stats()).is_empty());
}

#[test]
fn chips_show_reply_count_and_average_latency() {
    let mut stats = no_stats();
    stats.insert("planner".into(), (3, 12_600));
    let agents = [agent("planner", ""), agent("coder", "")];
    let line = text(&roster_cells(80, 1, &agents, &[], &stats));
    assert!(
        line.contains("planner \u{00b7}3\u{00d7} 4.2s"),
        "got: {line}"
    );
    assert!(
        !line.contains("coder \u{00b7}"),
        "no stat until a reply: {line}"
    );
}

#[test]
fn chip_stat_is_empty_for_unseen_agents() {
    assert_eq!(chip_stat(&no_stats(), "planner"), "");
}
