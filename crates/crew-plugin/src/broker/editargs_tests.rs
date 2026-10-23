use serde_json::json;

/// Two copies of a line, as the two-copies probe's `grade.rs` has them.
const GRADE: &str =
    "fn body() {\n    let floor = 4.5;\n}\n\nfn large() {\n    let floor = 4.5;\n}\n";

fn file(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-editargs-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("grade.rs");
    std::fs::write(&path, GRADE).unwrap();
    path
}

/// `line` beside a one-edit list is that edit's: a probe sent
/// `{"edits": [{…}], "line": 15}` and was told the line appears twice.
#[test]
fn a_line_beside_one_edit_is_that_edits() {
    let path = file("one");
    let swap = json!({"old": "let floor = 4.5;", "new": "let floor = 3.0;"});
    let args = json!({"path": path, "edits": [swap], "line": 6});
    super::edit(&args).unwrap();
    let after = std::fs::read_to_string(&path).unwrap();
    let want = "fn body() {\n    let floor = 4.5;\n}\n\nfn large() {\n    let floor = 3.0;\n}\n";
    assert_eq!(after, want, "the second copy, as `line` said");
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

/// Beside several, it is no one edit's: refused, saying where it goes, and
/// nothing is written.
#[test]
fn a_line_beside_several_edits_says_where_it_goes() {
    let path = file("many");
    let swap = |new: &str| json!({"old": "let floor = 4.5;", "new": new});
    let args = json!({"path": path, "edits": [swap("a"), swap("b")], "line": 6});
    let err = super::edit(&args).unwrap_err();
    assert!(err.contains("go inside the edit they are for"), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), GRADE);
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}
