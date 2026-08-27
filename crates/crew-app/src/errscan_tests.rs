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
