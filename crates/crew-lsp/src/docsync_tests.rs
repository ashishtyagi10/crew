use super::*;
use serde_json::json;

fn caps(sync: Value) -> DocSync {
    DocSync::from_initialize(&json!({"capabilities": {"textDocumentSync": sync}}))
}

/// rust-analyzer's own answer: incremental changes, saves without the text.
#[test]
fn rust_analyzers_answer_takes_changes_and_saves_without_text() {
    let s = caps(json!({"openClose": true, "change": 2, "save": {}}));
    assert_eq!(
        s,
        DocSync {
            change: true,
            save: Some(false)
        }
    );
}

#[test]
fn include_text_is_honoured_and_a_bare_true_saves_without_it() {
    assert_eq!(
        caps(json!({"change": 1, "save": {"includeText": true}})).save,
        Some(true)
    );
    assert_eq!(caps(json!({"change": 1, "save": true})).save, Some(false));
    assert_eq!(caps(json!({"change": 1, "save": false})).save, None);
    assert_eq!(caps(json!({"change": 1})).save, None);
}

/// The old spelling is a bare kind; None (0) takes neither changes nor saves.
#[test]
fn a_bare_kind_is_read_the_way_vs_code_reads_it() {
    for kind in [1, 2] {
        assert_eq!(
            caps(json!(kind)),
            DocSync {
                change: true,
                save: Some(false)
            },
            "kind {kind}"
        );
    }
    assert_eq!(caps(json!(0)), DocSync::default());
    assert!(!caps(json!({"openClose": true, "change": 0})).change);
}

/// An answer that never mentions sync means None: the spec's default.
#[test]
fn a_server_that_says_nothing_takes_no_changes() {
    assert_eq!(
        DocSync::from_initialize(&json!({"capabilities": {}})),
        DocSync {
            change: false,
            save: None
        }
    );
}

#[test]
fn the_digest_tells_same_text_from_moved_on() {
    assert_eq!(digest("fn a() {}\n"), digest("fn a() {}\n"));
    assert_ne!(digest("fn a() {}\n"), digest("fn b() {}\n"));
}
