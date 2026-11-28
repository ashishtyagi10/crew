#![cfg(unix)]
//! `lsp:symbols` against a scripted server: `sh` replays the handshake and
//! one canned `workspace/symbol` reply, then copies what crew sent to a file.
use super::*;
use crew_lsp::framing::{decode, encode};
use crew_lsp::servers::Server;
use crew_lsp::Symbol;
use std::collections::BTreeMap;
use std::path::Path;

fn sym(name: &str, kind: &'static str, line: u32) -> Symbol {
    Symbol {
        name: name.into(),
        kind,
        uri: crew_lsp::uri::from_path(Path::new("/p/src/a.rs")),
        line: Some(line),
    }
}

#[test]
fn lines_are_kind_name_path_line_best_first() {
    let root = PathBuf::from("/p");
    let found = vec![
        (root.clone(), sym("MyToolBudget", "struct", 4)),
        (root.clone(), sym("ToolBudget", "struct", 9)),
    ];
    assert_eq!(
        text("ToolBudget", found),
        "struct ToolBudget \u{2014} src/a.rs:10\nstruct MyToolBudget \u{2014} src/a.rs:5"
    );
}

#[test]
fn a_symbol_without_a_range_prints_the_file_alone() {
    let mut s = sym("f", "function", 0);
    s.line = None;
    assert_eq!(
        text("f", vec![("/p".into(), s)]),
        "function f \u{2014} src/a.rs"
    );
}

#[test]
fn thirty_lines_then_a_count() {
    let found: Vec<_> = (0..45)
        .map(|i| ("/p".into(), sym(&format!("Tool{i}"), "class", i)))
        .collect();
    let out = text("Tool", found);
    assert_eq!(out.lines().count(), 31);
    assert_eq!(out.lines().last(), Some("+15 more"));
    assert!(out.starts_with("class Tool0 \u{2014} src/a.rs:1\n"));
}

#[test]
fn nothing_found_says_so() {
    assert!(text("Nope", vec![]).starts_with("no symbols match \"Nope\""));
}

#[test]
fn two_servers_naming_one_symbol_print_it_once() {
    let found = vec![
        ("/p".into(), sym("ToolBudget", "struct", 1)),
        ("/p/src".into(), sym("ToolBudget", "struct", 1)),
    ];
    assert_eq!(text("ToolBudget", found).lines().count(), 1);
}

#[test]
fn each_path_is_relative_to_its_own_servers_root() {
    let mut other = sym("Tb", "struct", 0);
    other.uri = crew_lsp::uri::from_path(Path::new("/q/lib/b.rs"));
    let found = vec![("/p".into(), sym("Ta", "struct", 0)), ("/q".into(), other)];
    let out = text("T", found);
    assert!(
        out.contains("src/a.rs:1") && out.contains("lib/b.rs:1"),
        "{out}"
    );
}

#[test]
fn arguments_need_a_query_and_file_is_optional() {
    assert_eq!(
        parse(r#"{"query": " Foo ", "file": "x.rs"}"#).unwrap(),
        ("Foo".to_string(), Some("x.rs".to_string()))
    );
    assert_eq!(parse(r#"{"query": "Foo"}"#).unwrap().1, None);
    assert!(parse("{}").unwrap_err().contains("\"query\""));
    assert!(parse(r#"{"query": "  "}"#).is_err());
}

/// A host over a `sh` server answering the handshake, then `reply(dir)` to
/// the first request after it; the frames crew sends land in `heard`.
fn scripted(tag: &str, reply: impl Fn(&Path) -> Value) -> (LspHost, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("crew-lspsym-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"p\"\n").unwrap();
    let file = dir.join("src/lib.rs");
    std::fs::write(&file, "pub struct ToolBudget;\n").unwrap();
    let mut canned = encode(&json!({"jsonrpc": "2.0", "id": 1, "result": {"capabilities": {}}}));
    canned.extend(encode(
        &json!({"jsonrpc": "2.0", "id": 2, "result": reply(&dir)}),
    ));
    std::fs::write(dir.join("canned"), canned).unwrap();
    let heard = dir.join("heard");
    let script = format!(
        "exec 3<&0; cat <&3 > '{}' & cat '{}'; wait",
        heard.display(),
        dir.join("canned").display()
    );
    let server = Server {
        command: "sh".into(),
        args: vec!["-c".into(), script],
    };
    let host = LspHost::new(BTreeMap::from([("rust".to_string(), server)]));
    (host, file, heard)
}

/// The first `method` frame the server heard; the copy runs beside the
/// script, so it is polled for rather than read once.
fn sent(heard: &Path, method: &str) -> Option<Value> {
    for _ in 0..250 {
        if let Some(m) = heard_once(heard, method) {
            return Some(m);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    None
}

fn heard_once(heard: &Path, method: &str) -> Option<Value> {
    let bytes = std::fs::read(heard).unwrap_or_default();
    let mut r = std::io::BufReader::new(bytes.as_slice());
    std::iter::from_fn(|| decode(&mut r).ok().flatten()).find(|m| m["method"] == method)
}

fn uri(dir: &Path, rel: &str) -> String {
    crew_lsp::uri::from_path(&dir.join(rel))
}

#[test]
fn asks_workspace_symbol_and_merges_both_result_shapes() {
    let (mut host, file, heard) = scripted("ask", |d| {
        json!([
            {"name": "ToolBudgetX", "kind": 5, "location": {"uri": uri(d, "src/x.rs")}},
            {"name": "ToolBudget", "kind": 23, "location": {"uri": uri(d, "src/lib.rs"),
                "range": {"start": {"line": 0, "character": 11}, "end": {"line": 0, "character": 21}}}},
        ])
    });
    let args = json!({"query": "ToolBudget", "file": file}).to_string();
    let out = host.call("symbols", &args).unwrap();
    assert_eq!(
        out,
        "struct ToolBudget \u{2014} src/lib.rs:1\nclass ToolBudgetX \u{2014} src/x.rs"
    );
    let req = sent(&heard, "workspace/symbol").expect("request sent");
    assert_eq!(req["params"], json!({"query": "ToolBudget"}));
}

#[test]
fn a_null_reply_is_no_match() {
    let (mut host, file, _heard) = scripted("null", |_| Value::Null);
    let args = json!({"query": "Zzz", "file": file}).to_string();
    let out = host.call("symbols", &args).unwrap();
    assert!(out.starts_with("no symbols match \"Zzz\""), "{out}");
}

#[test]
fn without_file_or_server_it_says_how_to_get_one() {
    let mut h = LspHost::new(BTreeMap::new());
    let e = h.call("symbols", r#"{"query": "X"}"#).unwrap_err();
    assert!(
        e.contains("no language server is running") && e.contains("\"file\""),
        "{e}"
    );
    let e = h
        .call("symbols", r#"{"query": "X", "file": "notes.md"}"#)
        .unwrap_err();
    assert!(e.contains("no language server for .md files"), "{e}");
}

#[test]
fn a_missing_binary_is_named() {
    let mut h = LspHost::new(BTreeMap::from([(
        "rust".to_string(),
        Server {
            command: "crew-no-such-ls".into(),
            args: vec![],
        },
    )]));
    let e = h
        .call("symbols", r#"{"query": "X", "file": "src/lib.rs"}"#)
        .unwrap_err();
    assert!(
        e.contains("not installed") && e.contains("crew-no-such-ls"),
        "{e}"
    );
}
