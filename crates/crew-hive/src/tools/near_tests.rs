use super::*;

#[test]
fn edit_distance_is_the_textbook_one() {
    assert_eq!(levenshtein("kitten", "sitting"), 3);
    assert_eq!(levenshtein("", "abc"), 3);
    assert_eq!(levenshtein("same", "same"), 0);
}

#[test]
fn nearest_ranks_by_distance_and_refuses_the_far() {
    let names = [
        "weather__current",
        "weather__forecast",
        "sys__run",
        "sys__read_file",
        "gh__pr_list",
    ];
    assert_eq!(
        nearest(&names, "sys_run"),
        vec!["sys__run", "sys__read_file"]
    );
    assert_eq!(
        nearest(&names, "weather_curent"),
        vec!["weather__current", "weather__forecast"]
    );
    assert!(nearest(&names, "zzzzzzzz").is_empty(), "nothing is near");
    assert_eq!(
        nearest(&names, "SYS__RUN"),
        vec!["sys__run", "sys__read_file"],
        "case is not a difference"
    );
}

#[test]
fn a_short_list_is_simply_listed_and_a_long_one_says_what_was_meant() {
    let few = ["run", "list_dir", "read_file"];
    assert_eq!(
        unknown("sys tool", "rnu", &few, ""),
        "unknown sys tool \u{201c}rnu\u{201d} \u{2014} available: list_dir, read_file, run"
    );
    let many: Vec<String> = (0..20)
        .map(|i| format!("tool_{i:02}"))
        .chain(["sys__run".into()])
        .collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    let s = unknown("tool", "sys_run", &many, "; sys__find_tools searches them");
    assert_eq!(
        s,
        "unknown tool \u{201c}sys_run\u{201d} \u{2014} did you mean sys__run? (21 tools in all; sys__find_tools searches them)"
    );
    let s = unknown("tool", "qqqqqq", &many, "");
    assert_eq!(
        s,
        "unknown tool \u{201c}qqqqqq\u{201d} \u{2014} (21 tools in all)"
    );
    assert_eq!(
        unknown("MCP server", "ghost", &[], ""),
        "unknown MCP server \u{201c}ghost\u{201d} \u{2014} none available"
    );
}
