use super::*;

const SRC: &str = "fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}\n";

#[test]
fn a_unique_match_is_replaced_and_nothing_else_moves() {
    let out = replace(SRC, "let x = 1;", "let x = 2;").unwrap();
    assert_eq!(
        out,
        "fn main() {\n    let x = 2;\n    println!(\"{x}\");\n}\n"
    );
}

#[test]
fn a_repeated_match_is_refused_rather_than_guessed_at() {
    // Editing the first of two is a coin flip that succeeds silently, which
    // is the worst of the three possible outcomes.
    let body = "a = 1\nb = 2\na = 1\n";
    let err = replace(body, "a = 1", "a = 3").unwrap_err();
    assert!(err.starts_with("`old` appears 2 times"), "{err}");
    assert!(err.contains("surrounding lines"), "no way out: {err}");
}

#[test]
fn a_miss_on_whitespace_says_it_was_whitespace() {
    // The commonest miss by far: the model retyped the line without its
    // indentation. "not found" would send it looking for the wrong problem.
    let err = replace(SRC, "let x = 1;\nprintln!(\"{x}\");", "let x = 2;").unwrap_err();
    assert!(err.contains("different whitespace"), "{err}");
    assert!(err.contains("copy the indentation"), "{err}");
}

#[test]
fn a_miss_that_starts_right_says_where_it_diverged() {
    let err = replace(SRC, "    let x = 1;\n    let y = 2;", "x").unwrap_err();
    assert!(err.contains("let x = 1;"), "{err}");
    assert!(err.contains("does not match what follows"), "{err}");
}

#[test]
fn a_miss_with_nothing_in_common_says_so_plainly() {
    let err = replace(SRC, "struct Nothing;", "x").unwrap_err();
    assert!(
        err.starts_with("`old` is not in the file \u{2014}"),
        "{err}"
    );
    assert!(err.contains("sys:read_file"), "no next step: {err}");
}

#[test]
fn a_long_fragment_is_clipped_inside_the_error() {
    let long = "x".repeat(200);
    let body = format!("{long}\ntail\n");
    let err = replace(&body, &format!("{long}\nnope"), "y").unwrap_err();
    assert!(err.chars().count() < 200, "error is a wall of text: {err}");
    assert!(err.contains('\u{2026}'), "no clip marker: {err}");
}

#[test]
fn the_report_names_the_line_and_how_the_file_changed() {
    assert_eq!(
        report("/tmp/a/main.rs", SRC, "let x = 1;", "let x = 2;"),
        "edited main.rs at line 2 (1 line)"
    );
    assert_eq!(
        report("main.rs", SRC, "let x = 1;", "let x = 2;\n    let y = 3;"),
        "edited main.rs at line 2 (1 line \u{2192} 2, +1)"
    );
    assert_eq!(
        report(
            "main.rs",
            SRC,
            "    let x = 1;\n    println!(\"{x}\");\n",
            "\n"
        ),
        "edited main.rs at line 2 (2 lines \u{2192} 1, -1)"
    );
}

#[test]
fn an_empty_or_unchanged_old_never_touches_the_disk() {
    // Both would otherwise be a successful no-op write on a path that may not
    // even be the file the model meant.
    let err = edit("/nonexistent/nope.rs", "", "x").unwrap_err();
    assert!(err.contains("sys:write_file"), "{err}");
    let err = edit("/nonexistent/nope.rs", "same", "same").unwrap_err();
    assert!(err.contains("identical"), "{err}");
}

#[test]
fn an_edit_round_trips_through_a_real_file() {
    let dir = std::env::temp_dir().join(format!("crew-edit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.rs");
    std::fs::write(&path, SRC).unwrap();
    let p = path.to_str().unwrap();
    let out = edit(p, "let x = 1;", "let x = 42;").unwrap();
    assert_eq!(out, "edited main.rs at line 2 (1 line)");
    assert!(std::fs::read_to_string(&path)
        .unwrap()
        .contains("let x = 42;"));

    // A failed edit leaves the file exactly as it was.
    let before = std::fs::read_to_string(&path).unwrap();
    assert!(edit(p, "not here at all", "y").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_file_names_the_path_and_the_reason() {
    let err = edit("/nonexistent/really/nope.rs", "a", "b").unwrap_err();
    assert!(
        err.starts_with("edit /nonexistent/really/nope.rs:"),
        "{err}"
    );
}

fn swaps<'a>(pairs: &'a [(&'a str, &'a str)]) -> Vec<Swap<'a>> {
    pairs.iter().map(|(o, n)| Swap { old: o, new: n }).collect()
}

fn file(tag: &str, body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crew-editall-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.rs");
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn several_edits_land_in_one_call() {
    let path = file("many", SRC);
    let p = path.to_str().unwrap();
    let out = edit_all(
        p,
        &swaps(&[("let x = 1;", "let x = 2;"), ("fn main", "fn run")]),
    )
    .unwrap();
    assert_eq!(out, "edited main.rs in 2 places");
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(
        after.contains("let x = 2;") && after.contains("fn run("),
        "{after}"
    );
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn one_bad_edit_leaves_the_file_exactly_as_it_was() {
    // The whole reason the batch exists at all: half a rename does not
    // compile, and the model sent to fix it is reading a file that matches
    // neither what it read nor what it meant to write.
    let path = file("atomic", SRC);
    let p = path.to_str().unwrap();
    let err = edit_all(
        p,
        &swaps(&[("let x = 1;", "let x = 2;"), ("not here at all", "y")]),
    )
    .unwrap_err();
    assert!(err.contains("edit 2 of 2"), "{err}");
    assert!(err.contains("nothing was written"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        SRC,
        "it wrote anyway"
    );
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn edits_apply_in_order_so_one_can_set_up_the_next() {
    // Each swap runs against the result of the last, which is what lets a
    // rename be followed by an edit to the renamed line.
    let out = {
        let path = file("order", "let a = 1;\n");
        let p = path.to_str().unwrap();
        let out = edit_all(
            p,
            &swaps(&[("let a", "let b"), ("let b = 1;", "let b = 2;")]),
        )
        .unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
        (out, after)
    };
    assert_eq!(out.1, "let b = 2;\n");
}

#[test]
fn an_edit_whose_target_an_earlier_one_destroyed_says_so_plainly() {
    let path = file("gone", "let a = 1;\n");
    let p = path.to_str().unwrap();
    let err = edit_all(p, &swaps(&[("let a", "let b"), ("let a = 1;", "x")])).unwrap_err();
    assert!(err.contains("edit 2 of 2"), "{err}");
    assert!(err.contains("not in the file"), "{err}");
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn a_single_edit_still_reports_the_line_and_says_nothing_about_numbering() {
    // "edit 1 of 1" is noise, and the one-edit report is the one people read
    // most: it must not have changed at all.
    let path = file("single", SRC);
    let p = path.to_str().unwrap();
    assert_eq!(
        edit(p, "let x = 1;", "let x = 2;").unwrap(),
        "edited main.rs at line 2 (1 line)"
    );
    let err = edit(p, "nope", "y").unwrap_err();
    assert!(!err.contains("of 1"), "{err}");
    std::fs::remove_dir_all(path.parent().unwrap()).ok();
}

#[test]
fn an_empty_edit_list_is_refused_before_the_disk_is_touched() {
    assert!(edit_all("/nonexistent/x.rs", &[])
        .unwrap_err()
        .contains("no edits"));
}
