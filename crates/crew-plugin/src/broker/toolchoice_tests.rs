//! The model decides, the parser reads the grammar, the scorer is the fallback. Most of these
//! are about the boundary between the three: what the model is shown, what of its reply is
//! believed, and exactly when the scorer answers instead.
use super::*;

fn tool(server: &str, name: &str, desc: &str) -> McpTool {
    McpTool {
        server: server.into(),
        name: name.into(),
        description: desc.into(),
        input_schema: serde_json::json!({"type": "object"}),
    }
}

/// `n` tools nobody asked for, none of them crew's own.
fn filler(n: usize) -> Vec<McpTool> {
    (0..n)
        .map(|i| tool("noise", &format!("thing{i}"), "does an unrelated thing"))
        .collect()
}

/// A crowded catalog with crew's own tools in the MIDDLE, so "sys first" is a real claim.
fn crowded() -> Vec<McpTool> {
    let mut c = filler(10);
    c.push(tool("sys", "run", "run a shell command\nsecond paragraph"));
    c.push(tool("sys", "read_file", "read a file"));
    c.extend(filler(BUDGET).into_iter().skip(10));
    c.push(tool("gcal", "events", "list calendar events"));
    c
}

fn labels(tools: &[McpTool]) -> Vec<String> {
    tools.iter().map(label).collect()
}

#[test]
fn the_prompt_lists_every_tool_once_sys_first_and_the_task_last() {
    let catalog = crowded();
    let p = prompt("what is on my calendar this afternoon", &catalog);
    for l in labels(&catalog) {
        let row = format!("\n{l} \u{2014} ");
        assert_eq!(p.matches(&row).count(), 1, "{l} is listed once: {p}");
    }
    let sys_at = p
        .find("\nsys:run \u{2014} run a shell command\n")
        .expect("first line only");
    let first_noise = p.find("\nnoise:thing0 \u{2014} ").unwrap();
    assert!(sys_at < first_noise, "crew's own tools lead the list");
    assert!(p.contains("`TOOLS: <server:tool>"), "{p}");
    assert!(p.ends_with("Task: what is on my calendar this afternoon"));
    assert!(!p.contains("more not listed"), "nothing was cut: {p}");
}

#[test]
fn the_listing_stops_at_the_byte_cap_and_counts_what_did_not_fit() {
    let wide = "x".repeat(DESC_CAP);
    let catalog: Vec<McpTool> = (0..400)
        .map(|i| tool("big", &format!("t{i:03}"), &wide))
        .collect();
    let p = prompt("anything", &catalog);
    let rows = p
        .split("Tools:\n")
        .nth(1)
        .unwrap()
        .split("\n\u{2026} ")
        .next()
        .unwrap();
    assert!(rows.len() <= LIST_CAP, "{} bytes of rows", rows.len());
    let shown = rows.lines().count();
    assert!(shown < 400 && shown > 100, "{shown} rows fitted");
    let marker = format!("\n\u{2026} {} more not listed", 400 - shown);
    assert!(p.contains(&marker), "{p:.200}");
}

#[test]
fn a_tools_line_keeps_exact_names_in_catalog_order_dropping_unknowns_and_dupes() {
    let catalog = crowded();
    let names = parse(
        "TOOLS: gcal:events, noise:thing3, gcal:events, nope:nothing, sys:run, Noise:Thing4",
        &catalog,
    );
    assert_eq!(
        names.unwrap(),
        ["noise:thing3", "sys:run", "gcal:events"],
        "catalog order, one each, exact spelling"
    );
    let too_many: Vec<String> = (0..BUDGET + 3).map(|i| format!("noise:thing{i}")).collect();
    let kept = parse(&format!("TOOLS: {}", too_many.join(", ")), &crowded()).unwrap();
    assert_eq!(
        kept.len(),
        BUDGET,
        "at most the budget, the first named win"
    );
}

#[test]
fn tools_none_is_an_empty_choice_and_an_off_grammar_reply_is_no_choice() {
    let catalog = crowded();
    assert_eq!(parse("TOOLS: none", &catalog), Some(vec![]));
    assert_eq!(parse("\n**tools: NONE**\n", &catalog), Some(vec![]));
    assert_eq!(parse("I would use gcal:events", &catalog), None);
    assert_eq!(
        parse("TOOLS: nope:a, nope:b", &catalog),
        None,
        "nothing known"
    );
    assert_eq!(parse("", &catalog), None);
}

#[test]
fn the_choice_is_the_named_tools_plus_every_sys_tool_in_catalog_order() {
    let catalog = crowded();
    let chooser = |_: &str| Ok("TOOLS: gcal:events, noise:thing7".to_string());
    let (kept, left_out) = choose(&catalog, "calendar", Some(&chooser)).expect("a choice");
    assert_eq!(
        labels(&kept),
        ["noise:thing7", "sys:run", "sys:read_file", "gcal:events"]
    );
    assert_eq!(left_out, catalog.len() - 4);
    let none = |_: &str| Ok("TOOLS: none".to_string());
    let (kept, _) = choose(&catalog, "calendar", Some(&none)).expect("a choice");
    assert_eq!(
        labels(&kept),
        ["sys:run", "sys:read_file"],
        "none = sys only"
    );
}

#[test]
fn a_failed_call_an_off_grammar_reply_and_an_under_budget_catalog_are_no_choice() {
    let failing = |_: &str| Err("boom".to_string());
    assert_eq!(choose(&crowded(), "x", Some(&failing)), None);
    let rambling = |_: &str| Ok("Sure! I'd pick a few.".to_string());
    assert_eq!(choose(&crowded(), "x", Some(&rambling)), None);
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let counting = |_: &str| {
        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok("TOOLS: noise:thing1".to_string())
    };
    assert_eq!(choose(&filler(BUDGET), "x", Some(&counting)), None);
    let asked = calls.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(asked, 0, "under the budget the model is not asked");
    assert_eq!(choose(&crowded(), "x", None), None);
}

#[test]
fn the_pane_line_names_what_was_chosen_and_cuts_a_long_list() {
    let short = Chosen {
        of: 41,
        names: vec![
            "sys:run".into(),
            "lsp:hover".into(),
            "github:search_issues".into(),
        ],
    };
    assert_eq!(
        short.line(),
        "tools: chose 3 of 41 \u{2014} sys:run, lsp:hover, github:search_issues"
    );
    let long = Chosen {
        of: 120,
        names: (0..BUDGET)
            .map(|i| format!("workspace:tool_{i:02}"))
            .collect(),
    };
    let line = long.line();
    assert!(line.starts_with("tools: chose 24 of 120 \u{2014} workspace:tool_00, "));
    let shown = line.matches("workspace:").count();
    assert!(shown < BUDGET, "{line}");
    assert!(
        line.ends_with(&format!(", \u{2026} +{}", BUDGET - shown)),
        "{line}"
    );
    assert!(!line.contains('\n'));
}
