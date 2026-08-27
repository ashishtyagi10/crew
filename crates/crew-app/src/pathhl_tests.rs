use super::*;

fn spans(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    path_spans(&chars)
        .into_iter()
        .map(|(a, b)| chars[a..b].iter().collect())
        .collect()
}

#[test]
fn the_shapes_an_agent_actually_writes_are_marked() {
    assert_eq!(spans("see src/main.rs:42 for details"), ["src/main.rs:42"]);
    assert_eq!(spans("run ./deploy.sh now"), ["./deploy.sh"]);
    assert_eq!(spans("in ~/notes/todo.md"), ["~/notes/todo.md"]);
    assert_eq!(spans("edit Cargo.toml"), ["Cargo.toml"]);
    assert_eq!(spans("at /etc/hosts:12:3"), ["/etc/hosts:12:3"]);
}

/// The whole risk of this feature: a mark on ordinary prose teaches people to
/// ignore the marks.
#[test]
fn prose_that_merely_contains_a_slash_or_a_dot_is_not_marked() {
    for line in [
        "and/or is not a path",
        "e.g. this sentence",
        "TCP/IP and I/O",
        "it took 10:30 to run",
        "version v1.0 shipped",
        "see Fig.2 above",
        "yes/no?",
        "a.b",
        "--flag=x",
    ] {
        assert!(spans(line).is_empty(), "{line} → {:?}", spans(line));
    }
}

/// URLs belong to `linkhl`; marking them here as well would give one span two
/// different rules.
#[test]
fn a_url_is_not_a_file_reference() {
    assert!(spans("https://example.com/a.rs").is_empty());
    assert!(spans("see http://x.io/y.md now").is_empty());
}

/// Prose punctuation around a reference is not part of it — the click has to
/// resolve `src/main.rs`, not `src/main.rs).`
#[test]
fn surrounding_punctuation_is_left_out_of_the_span() {
    assert_eq!(spans("(see src/main.rs)."), ["src/main.rs"]);
    assert_eq!(spans("\"lib/x.rs\","), ["lib/x.rs"]);
    assert_eq!(spans("`Cargo.toml`:"), ["Cargo.toml"]);
}

#[test]
fn a_position_suffix_is_split_off_the_path() {
    assert_eq!(strip_position("src/main.rs"), ("src/main.rs", None));
    assert_eq!(strip_position("src/main.rs:42"), ("src/main.rs", Some(42)));
    // Column too: the LINE is what a viewer can act on, and it is the first
    // number, not the last.
    assert_eq!(
        strip_position("src/main.rs:42:7"),
        ("src/main.rs", Some(42))
    );
    // Not a position: a Windows drive letter, a time, a bare colon.
    assert_eq!(strip_position("C:/tmp/x.rs"), ("C:/tmp/x.rs", None));
    assert_eq!(strip_position("x.rs:"), ("x.rs:", None));
}

/// Several references on one line all get marked, each on its own.
#[test]
fn every_reference_on_a_line_is_found() {
    assert_eq!(
        spans("moved src/a.rs to src/b.rs (see also docs/x.md:9)"),
        ["src/a.rs", "src/b.rs", "docs/x.md:9"]
    );
}

/// On the grid: the cells of a reference take the link colour and a DOTTED
/// rule — a different affordance from a URL's solid one, because it opens
/// here rather than leaving for a browser.
#[test]
fn marked_cells_take_the_link_colour_and_a_dotted_rule() {
    let _g = crate::app::theme_test_guard();
    let line = "open src/main.rs now";
    let mut cells: Vec<CellView> = line
        .chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: 0,
            c,
            fg: (200, 200, 200),
            ..Default::default()
        })
        .collect();
    let n = mark(&mut cells, line.len() as u16, 1);
    assert_eq!(n, "src/main.rs".len());
    let at = |i: usize| cells.iter().find(|c| c.col == i as u16).unwrap();
    let start = line.find("src").unwrap();
    assert_eq!(at(start).deco.line, DecoLine::Dotted);
    assert_eq!(at(start).fg, crate::linkhl::link_fg());
    assert_eq!(at(0).deco.line, DecoLine::None, "prose was marked");
    assert_ne!(at(0).fg, crate::linkhl::link_fg());
}
