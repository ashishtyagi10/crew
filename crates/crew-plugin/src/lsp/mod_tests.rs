use super::*;
use crew_lsp::{Location, Position, Range, Severity};

fn table(cmd: &str) -> BTreeMap<String, Server> {
    BTreeMap::from([(
        "rust".to_string(),
        Server {
            command: cmd.into(),
            args: vec![],
        },
    )])
}

#[test]
fn the_four_tools_are_listed_only_when_a_server_is_configured() {
    assert!(LspHost::default().tools().is_empty());
    let names: Vec<String> = LspHost::new(table("rust-analyzer"))
        .tools()
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, ["hover", "definition", "references", "diagnostics"]);
}

/// Every descriptor is one the picker can find for a code task: the words
/// a task about code uses appear on the FIRST line, which is all it scores.
#[test]
fn descriptions_lead_with_the_words_a_code_task_uses() {
    for t in tools::descriptors() {
        let first = t.description.lines().next().unwrap_or("").to_lowercase();
        assert!(first.contains("language server"), "{}: {first}", t.name);
        assert_eq!(t.server, "lsp");
        assert_eq!(t.input_schema["required"][0], "file");
        assert_eq!(t.input_schema["type"], "object");
    }
    let hover = &tools::descriptors()[0];
    assert_eq!(hover.input_schema["required"][1], "line");
    assert_eq!(hover.input_schema["properties"]["col"]["minimum"], 1);
}

#[test]
fn args_are_one_based_and_default_to_the_first_character() {
    let a = Args::parse(r#"{"file": "src/main.rs", "line": 12, "col": 8}"#).unwrap();
    assert_eq!(
        a,
        Args {
            file: "src/main.rs".into(),
            line: 12,
            col: 8
        }
    );
    let a = Args::parse(r#"{"file": "x.rs"}"#).unwrap();
    assert_eq!((a.line, a.col), (1, 1));
    assert_eq!(
        (Args::parse(r#"{"file": "x.rs", "line": 0}"#).unwrap().line),
        1
    );
    assert!(Args::parse("{}").unwrap_err().contains("\"file\""));
    assert!(Args::parse("nope").unwrap_err().contains("not valid JSON"));
}

/// The message names the binary, because that is the thing to install.
#[test]
fn a_missing_server_says_not_installed_and_names_it() {
    let mut h = LspHost::new(table("crew-no-such-ls"));
    let e = h
        .call("hover", r#"{"file": "src/lib.rs", "line": 1, "col": 1}"#)
        .unwrap_err();
    assert!(e.contains("not installed"), "{e}");
    assert!(e.contains("crew-no-such-ls"), "{e}");
}

#[test]
fn a_language_nobody_serves_is_refused_before_any_spawn() {
    let mut h = LspHost::new(table("rust-analyzer"));
    let e = h.call("hover", r#"{"file": "notes.md"}"#).unwrap_err();
    assert!(e.contains("no language server for .md files"), "{e}");
    let e = h.call("hover", r#"{"file": "x.go"}"#).unwrap_err();
    assert!(e.contains("no language server configured for go"), "{e}");
}

fn diag(line: u32, sev: Severity, msg: &str, src: Option<&str>) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line, character: 4 },
            end: Position { line, character: 9 },
        },
        severity: sev,
        message: msg.into(),
        source: src.map(str::to_string),
    }
}

#[test]
fn diagnostics_read_as_path_line_col_severity_message() {
    let root = Path::new("/p");
    let path = Path::new("/p/src/main.rs");
    let text = tools::diagnostics_text(
        root,
        path,
        &[
            diag(
                1,
                Severity::Error,
                "mismatched types\nexpected i32",
                Some("rustc"),
            ),
            diag(4, Severity::Warning, "unused variable", None),
        ],
    );
    assert_eq!(
        text,
        "src/main.rs:2:5 \u{2014} error [rustc]: mismatched types\nsrc/main.rs:5:5 \u{2014} warning: unused variable"
    );
    assert_eq!(
        tools::diagnostics_text(root, path, &[]),
        "no diagnostics for src/main.rs"
    );
}

#[test]
fn locations_and_hovers_are_project_relative() {
    let root = Path::new("/p");
    let path = Path::new("/p/src/main.rs");
    let a = Args {
        file: "src/main.rs".into(),
        line: 3,
        col: 9,
    };
    let loc = Location {
        uri: crew_lsp::uri::from_path(Path::new("/p/src/lib.rs")),
        range: Range {
            start: Position {
                line: 9,
                character: 3,
            },
            end: Position {
                line: 9,
                character: 8,
            },
        },
    };
    assert_eq!(
        tools::locations_text("definition", root, path, &a, &[loc]),
        "src/lib.rs:10:4"
    );
    assert_eq!(
        tools::locations_text("references", root, path, &a, &[]),
        "no references found for src/main.rs:3:9"
    );
    assert_eq!(
        tools::hover_text(root, path, &a, Some("fn main()".into())),
        "src/main.rs:3:9 \u{2014} fn main()"
    );
    assert!(tools::hover_text(root, path, &a, None).starts_with("no hover information"));
}

#[test]
fn status_rows_say_which_servers_are_installed() {
    let mut t = table("crew-no-such-ls");
    t.insert(
        "shell".into(),
        Server {
            command: if cfg!(windows) { "cmd" } else { "sh" }.into(),
            args: vec![],
        },
    );
    let rows = LspHost::new(t).status_rows();
    assert_eq!(
        rows[0],
        ("rust".to_string(), "crew-no-such-ls".to_string(), false)
    );
    assert_eq!(rows[1].0, "shell");
    assert!(rows[1].2);
}

/// A server that will not start is a `Status` line for the LOG — once, not
/// once per call, since every call re-tries — and never a `Loaded` line.
#[test]
fn a_server_that_will_not_start_is_noted_once_and_never_announced_loaded() {
    use std::sync::{Arc, Mutex};
    let mut h = LspHost::new(table("crew-no-such-ls"));
    let notes: Arc<Mutex<Vec<(bool, String)>>> = Arc::default();
    let loaded: Arc<Mutex<Vec<crew_lsp::Diagnostic>>> = Arc::default();
    let n = Arc::clone(&notes);
    let l = Arc::clone(&loaded);
    h.set_sinks(
        Arc::new(move |e, m: &str| n.lock().unwrap().push((e, m.to_string()))),
        Arc::new(move |_| l.lock().unwrap().clear()),
    );
    for _ in 0..2 {
        h.call("hover", r#"{"file": "src/lib.rs", "line": 1, "col": 1}"#)
            .unwrap_err();
    }
    let notes = notes.lock().unwrap();
    assert_eq!(notes.len(), 1, "one note for two tries: {notes:?}");
    assert!(notes[0].0, "an error-level note");
    assert!(
        notes[0].1.starts_with("lsp crew-no-such-ls: "),
        "names the command: {}",
        notes[0].1
    );
    assert!(notes[0].1.contains("not installed"), "{}", notes[0].1);
}

/// The `Loaded` line: the command (what you would install), the language,
/// the project directory's NAME — not its whole path, which would eat the row.
#[test]
fn the_loaded_line_names_command_language_and_project() {
    let ev = loaded_event("rust", "rust-analyzer", Path::new("/Users/me/code/crew"));
    assert_eq!(
        ev,
        crew_hive::HiveEvent::Loaded {
            agent: String::new(),
            kind: "lsp".into(),
            name: "rust-analyzer".into(),
            detail: "rust \u{b7} crew".into(),
        }
    );
}
