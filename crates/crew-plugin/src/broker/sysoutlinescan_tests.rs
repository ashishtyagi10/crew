use super::*;

fn rust(src: &str) -> Vec<String> {
    code_lines(src, Flavor::Rust)
}

#[test]
fn a_string_keeps_its_quotes_and_loses_its_braces_and_a_line_comment_goes() {
    assert_eq!(
        rust("let s = \"fn a() { \\\" }\"; // fn b() {"),
        ["let s = \"\"; "]
    );
}

#[test]
fn a_string_left_open_runs_on_to_the_line_that_closes_it() {
    assert_eq!(
        rust("const HELP: &str = \"\nfn not_a_function() {\n\";\nfn after() {}"),
        ["const HELP: &str = \"", "", "\";", "fn after() {}"]
    );
}

#[test]
fn a_raw_string_ends_only_at_its_own_hashes() {
    assert_eq!(
        rust("let x = r#\"a \"{\" b\"#; let y = br\"}\";"),
        ["let x = \"\"; let y = b\"\";"]
    );
    // `r#type` is a raw identifier, and an `r` ending a name starts nothing.
    assert_eq!(rust("r#type(bar\"{\")"), ["r#type(bar\"\")"]);
}

#[test]
fn a_char_literal_is_blanked_and_a_lifetime_is_left_alone() {
    assert_eq!(
        rust("f('{', '\\'', '\\u{7d}', &'a x, 'outer: loop {"),
        ["f('', '', '', &'a x, 'outer: loop {"]
    );
}

#[test]
fn block_comments_nest_in_rust_and_run_over_lines() {
    assert_eq!(
        rust("a /* x /* y */ { */ b {\nc /* {\n} */ d"),
        ["a   b {", "c ", "  d"]
    );
}

#[test]
fn clike_quotes_end_with_their_line_and_a_backtick_string_runs_on() {
    let code = code_lines("x = 'a { b\ny = `t {\n} u` + \"{\"\nz {", Flavor::CLike);
    assert_eq!(code, ["x = '", "y = `", "` + \"\"", "z {"]);
}
