#![cfg(unix)]
//! The server's copy of a file follows the disk: an edit between two calls
//! reaches the server whole, at the next version, with a save; no edit sends
//! nothing. The server is the scripted peer in `peer_tests.rs`.
use super::peer::{names, rust_analyzer, Peer};
use serde_json::{json, Value};
use std::time::Duration;

#[test]
fn an_edit_on_disk_reaches_the_server_whole_at_version_2_then_a_save() {
    let mut p = Peer::new("edit", rust_analyzer());
    let opened = p.hover();
    assert_eq!(
        names(&opened),
        ["textDocument/didOpen"],
        "a hover saves nothing"
    );
    assert_eq!(opened[0]["params"]["textDocument"]["version"], 1);
    p.write("fn b() {}\n");
    let heard = p.hover();
    assert_eq!(
        names(&heard[1..]),
        ["textDocument/didChange", "textDocument/didSave"]
    );
    assert_eq!(heard[1]["params"]["textDocument"]["version"], 2);
    assert_eq!(
        heard[1]["params"]["contentChanges"],
        json!([{"text": "fn b() {}\n"}])
    );
    assert_eq!(
        heard[2]["params"],
        json!({"textDocument": {"uri": p.uri()}})
    );
}

#[test]
fn a_file_that_has_not_moved_on_sends_nothing() {
    let mut p = Peer::new("same", rust_analyzer());
    let opened = p.hover();
    assert_eq!(p.hover(), opened, "a second look at the same text");
    p.write("fn b() {}\n");
    let changed = p.hover();
    assert_eq!(changed.len(), 3, "{changed:?}");
    assert_eq!(p.hover(), changed, "a second look after the change");
}

#[test]
fn three_edits_are_versions_2_3_and_4_in_order() {
    let mut p = Peer::new("three", rust_analyzer());
    p.hover();
    let mut heard = Vec::new();
    for text in ["fn b() {}\n", "fn c() {}\n", "fn d() {}\n"] {
        p.write(text);
        heard = p.hover();
    }
    let changes: Vec<(Value, Value)> = heard
        .iter()
        .filter(|m| m["method"] == "textDocument/didChange")
        .map(|m| {
            (
                m["params"]["textDocument"]["version"].clone(),
                m["params"]["contentChanges"][0]["text"].clone(),
            )
        })
        .collect();
    assert_eq!(
        changes,
        [
            (json!(2), json!("fn b() {}\n")),
            (json!(3), json!("fn c() {}\n")),
            (json!(4), json!("fn d() {}\n")),
        ]
    );
}

/// rust-analyzer runs `cargo check`, and so finds the type errors, only on a
/// save: a first look for diagnostics opens the file AND saves it.
#[test]
fn a_first_look_for_diagnostics_saves_so_the_checker_runs() {
    let mut p = Peer::scripted("diag", rust_analyzer(), &[(0.3, None)]);
    let out = p.host.call("diagnostics", &p.args()).unwrap();
    assert_eq!(out, "no diagnostics for src/lib.rs");
    let heard = p.dids_once(|f| f.iter().any(|m| m["method"] == "textDocument/didSave"));
    assert_eq!(
        names(&heard),
        ["textDocument/didOpen", "textDocument/didSave"]
    );
}

#[test]
fn a_server_that_takes_no_changes_is_closed_and_reopened_at_the_next_version() {
    let mut p = Peer::new("reopen", json!({}));
    p.hover();
    p.write("fn b() {}\n");
    let heard = p.hover();
    assert_eq!(
        names(&heard),
        [
            "textDocument/didOpen",
            "textDocument/didClose",
            "textDocument/didOpen"
        ],
        "no didChange and no didSave: it asked for neither"
    );
    assert_eq!(heard[2]["params"]["textDocument"]["version"], 2);
    assert_eq!(heard[2]["params"]["textDocument"]["text"], "fn b() {}\n");
}

#[test]
fn a_server_that_wants_the_text_on_save_gets_it() {
    let mut p = Peer::new(
        "text",
        json!({"textDocumentSync": {"change": 1, "save": {"includeText": true}}}),
    );
    p.hover();
    p.write("fn b() {}\n");
    let heard = p.hover();
    assert_eq!(heard[2]["method"], "textDocument/didSave");
    assert_eq!(heard[2]["params"]["text"], "fn b() {}\n");
}

/// What was queued when an edit went out was said about the text before it:
/// the call after the edit answers with the server's word on the new text.
#[test]
fn a_publish_queued_before_an_edit_does_not_answer_for_it() {
    let steps = [(0.4, Some("about the old text")), (0.8, None)];
    let mut p = Peer::scripted("queued", rust_analyzer(), &steps);
    p.hover();
    // The old text's publish lands while nobody is asking.
    std::thread::sleep(Duration::from_millis(800));
    p.write("fn b() {}\n");
    let out = p.host.call("diagnostics", &p.args()).unwrap();
    assert_eq!(out, "no diagnostics for src/lib.rs");
}
