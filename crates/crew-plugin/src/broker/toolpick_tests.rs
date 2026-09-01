//! Retrieval's risk is that the right tool is the one it drops, so most of these are about what
//! survives: crew's own tools, a tool the task names, and a way back to everything else.
use super::*;

fn tool(server: &str, name: &str, desc: &str) -> McpTool {
    McpTool {
        server: server.into(),
        name: name.into(),
        description: desc.into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

/// `n` plausible-looking tools nobody asked for, to push a catalog over the budget.
fn filler(n: usize) -> Vec<McpTool> {
    (0..n)
        .map(|i| tool("noise", &format!("thing{i}"), "does an unrelated thing"))
        .collect()
}

fn labels(tools: &[McpTool]) -> Vec<String> {
    tools
        .iter()
        .map(|t| format!("{}:{}", t.server, t.name))
        .collect()
}

#[test]
fn a_small_catalog_is_never_filtered() {
    // The behaviour crew has today has to survive this change untouched: below the budget the
    // prompt is exactly what it always was.
    let tools = vec![
        tool("sys", "run", "run a shell command"),
        tool("gcal", "events", "list calendar events"),
    ];
    let (kept, left) = pick(tools.clone(), "anything at all", BUDGET);
    assert_eq!(labels(&kept), labels(&tools));
    assert_eq!(left, 0);
    assert_eq!(omitted_note(left), "", "and it admits to hiding nothing");
}

#[test]
fn the_tools_the_task_is_about_are_the_ones_shown() {
    let mut tools = filler(40);
    tools.push(tool("gcal", "events", "list calendar events for a day"));
    tools.push(tool("weather", "forecast", "the forecast for a city"));
    let (kept, left) = pick(tools, "what is on my calendar tomorrow", 6);
    assert!(
        labels(&kept).contains(&"gcal:events".to_string()),
        "{:?}",
        labels(&kept)
    );
    assert_eq!(kept.len(), 6);
    assert_eq!(left, 36, "and the rest are counted, not forgotten");
}

#[test]
fn crews_own_tools_are_never_dropped_however_the_task_is_worded() {
    let mut tools = filler(60);
    tools.push(tool("sys", "run", "run a shell command"));
    tools.push(tool("sys", "read_file", "read a text file"));
    let (kept, _) = pick(tools, "book me a table at a restaurant", 8);
    let kept = labels(&kept);
    assert!(kept.contains(&"sys:run".to_string()), "{kept:?}");
    assert!(kept.contains(&"sys:read_file".to_string()), "{kept:?}");
}

#[test]
fn a_tool_the_task_names_outright_wins_over_everything() {
    let mut tools = filler(40);
    tools.push(tool("obscure", "zzz", "no words in common with anything"));
    let (kept, _) = pick(tools, "use @tool obscure:zzz to do it", 3);
    assert!(labels(&kept).contains(&"obscure:zzz".to_string()));
}

#[test]
fn a_name_outweighs_a_passing_mention_in_a_description() {
    let named = tool("x", "calendar", "does a thing");
    let mentions = tool("y", "thing", "useful when you have a calendar open");
    let words = words("check my calendar");
    let (a, b) = (
        score(&named, &words, "check my calendar"),
        score(&mentions, &words, "check my calendar"),
    );
    assert!(a > b, "named {a} should beat mentioned {b}");
}

#[test]
fn the_choice_is_the_same_twice_for_the_same_task() {
    // A prompt that reshuffles between hops is a prompt no cache can help and no bug can be
    // reproduced against.
    let mut tools = filler(50);
    tools.push(tool("gcal", "events", "list calendar events"));
    let once = labels(&pick(tools.clone(), "my calendar today", 10).0);
    let twice = labels(&pick(tools, "my calendar today", 10).0);
    assert_eq!(once, twice);
}

#[test]
fn what_was_left_out_is_admitted_with_a_number_and_a_way_back() {
    let note = omitted_note(37);
    assert!(note.contains("37"), "{note}");
    assert!(note.contains("sys:find_tools"), "{note}");
}

#[test]
fn a_search_finds_by_name_by_server_and_by_substring() {
    let tools = vec![
        tool("gcal", "events", "list calendar events for a day"),
        tool("weather", "forecast", "the forecast for a city"),
        tool("sys", "run", "run a shell command"),
    ];
    let hits = search(&tools, "calendar", 10);
    assert!(hits.contains("gcal:events"), "{hits}");
    assert!(!hits.contains("weather:forecast"), "{hits}");
    // "cal" is not a word of anything, but it is how somebody would search.
    assert!(search(&tools, "cal", 10).contains("gcal:events"));
    assert!(search(&tools, "weather", 10).contains("weather:forecast"));
}

#[test]
fn a_search_that_finds_nothing_says_so_rather_than_returning_an_empty_list() {
    let tools = vec![tool("gcal", "events", "list calendar events")];
    let out = search(&tools, "quantum tunnelling", 10);
    assert!(out.contains("no tool matches"), "{out}");
    assert!(out.contains('1'), "and says how many there are: {out}");
}

#[test]
fn an_empty_query_is_not_a_match_for_everything() {
    // `"".contains()` is true of every string, so a substring search on an empty query would
    // hand back the entire catalog — undoing the budget through a malformed call.
    let tools = vec![
        tool("gcal", "events", "list calendar events"),
        tool("sys", "run", "run a shell command"),
    ];
    let out = search(&tools, "   ", 10);
    assert!(out.contains("no tool matches"), "{out}");
    assert!(!out.contains("gcal:events"), "{out}");
}

#[test]
fn a_search_is_capped_so_one_question_cannot_undo_the_budget() {
    let tools: Vec<McpTool> = (0..50)
        .map(|i| tool("noise", &format!("calendar{i}"), "a calendar thing"))
        .collect();
    let out = search(&tools, "calendar", 5);
    assert_eq!(
        out.lines().filter(|l| l.starts_with("- ")).count(),
        5,
        "{out}"
    );
}
