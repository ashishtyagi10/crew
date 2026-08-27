use super::*;

#[test]
fn the_shapes_real_tools_print_are_errors() {
    for line in [
        "error[E0433]: failed to resolve: use of undeclared crate",
        "error: could not compile `crew-app`",
        "src/main.rs:4:1: error: expected one of `.`",
        "src/app.ts(12,5): error TS2345: Argument of type 'number'",
        "main.c:3:5: fatal error: stdio.h: No such file or directory",
        "fatal: not a git repository",
        "thread 'main' panicked at src/lib.rs:12:5:",
        "panicked at 'index out of bounds'",
        "npm ERR! code ELIFECYCLE",
        "Traceback (most recent call last):",
        "FAILED tests/test_x.py::test_y",
        "not ok 3 - the thing works",
        "\u{2717} renders the header",
        "  \u{2502} error: inside a TUI's box",
    ] {
        assert!(looks_like_error(line), "missed: {line}");
    }
}

/// The risk is the opposite one: a jump that lands on prose teaches you not
/// to trust the jump.
#[test]
fn prose_that_merely_mentions_an_error_is_not_one() {
    for line in [
        "errors are handled in the next section",
        "this fixes the error handling",
        "no errors found",
        "  let e = Error::new();",
        "warning: unused variable `x`",
        "0 errors, 2 warnings",
        "",
        "   ",
        "erroneous is a word",
    ] {
        assert!(!looks_like_error(line), "false positive: {line}");
    }
}

/// Case is not a signal: `ERROR:` and `error:` are the same line.
#[test]
fn matching_ignores_case() {
    assert!(looks_like_error("ERROR: build failed"));
    assert!(looks_like_error("Fatal: everything is on fire"));
    assert!(looks_like_error("FAILED (errors=1)"));
}

/// A line's leading chrome — indent, a quote bar, a TUI's box edge — is not
/// part of what it says.
#[test]
fn chrome_in_front_of_a_line_is_not_part_of_it() {
    assert!(looks_like_error("    error: indented"));
    assert!(looks_like_error("> error: quoted"));
    assert!(looks_like_error("\u{2503} error: in a box"));
    assert!(looks_like_error("\u{276f} error: after a prompt glyph"));
}

fn rows(lines: &[&str]) -> Vec<Vec<char>> {
    lines.iter().map(|l| l.chars().collect()).collect()
}

#[test]
fn only_the_rows_that_hold_an_error_are_marked() {
    let grid = rows(&[
        "$ cargo build",
        "   Compiling crew-app v0.1.0",
        "error[E0433]: failed to resolve",
        " --> src/main.rs:4:5",
        "warning: unused import",
        "error: could not compile",
    ]);
    assert_eq!(super::error_rows(&grid), vec![2, 5]);
    assert!(super::error_rows(&rows(&["all good", ""])).is_empty());
}

fn bar(err_rows: &[u16]) -> crate::panecard::Bar<'_> {
    crate::panecard::Bar {
        index: Some(1),
        title: "build",
        focused: false,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        assemble_t: 1.0,
        focus_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        cmd_rows: &[],
        err_rows,
        unread: 0,
        doc: false,
    }
}

/// On the card: the marks ride the LEFT border, one per error row, and never
/// on a corner.
#[test]
fn the_card_marks_error_rows_down_its_left_border() {
    let _g = crate::app::theme_test_guard();

    let marks = |rows: &[u16]| -> Vec<u16> {
        crate::panecard::pane_card(40, 8, &bar(rows))
            .into_iter()
            .filter(|c| c.col == 0 && c.c == '\u{258c}')
            .map(|c| c.row)
            .collect()
    };
    // Content row 0 is border row 1.
    assert_eq!(marks(&[0, 3]), vec![1, 4]);
    assert!(marks(&[]).is_empty());
    // A row past the card's own bottom is not drawn onto its corner.
    assert!(marks(&[99]).is_empty());
    let bell = crew_theme::theme().bell;
    assert!(crate::panecard::pane_card(40, 8, &bar(&[1]))
        .iter()
        .any(|c| c.col == 0 && c.c == '\u{258c}' && c.fg == bell));
}

/// Both marks live on the same border, and a row that is both the start of a
/// command and an error reads as the error: "this failed" outranks "this
/// began".
#[test]
fn an_error_outranks_a_command_start_on_the_same_row() {
    let _g = crate::app::theme_test_guard();
    let both = crate::panecard::Bar {
        cmd_rows: &[2],
        err_rows: &[2],
        ..bar(&[])
    };
    let at2: Vec<char> = crate::panecard::pane_card(40, 8, &both)
        .into_iter()
        .filter(|c| c.col == 0 && c.row == 3)
        .map(|c| c.c)
        .collect();
    assert_eq!(at2, vec!['\u{258c}'], "{at2:?}");
    let only_start = crate::panecard::Bar {
        cmd_rows: &[2],
        ..bar(&[])
    };
    assert!(crate::panecard::pane_card(40, 8, &only_start)
        .iter()
        .any(|c| c.col == 0 && c.row == 3 && c.c == '\u{2576}'));
}
