use super::*;

/// Every character of the input must survive tokenizing, in order. A lexer
/// that drops or reorders text would silently corrupt the code it renders,
/// which is worse than not highlighting at all.
fn assert_lossless(line: &str, lang: &str) -> Vec<(String, Token)> {
    let runs = tokenize(line, lang);
    let joined: String = runs.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(joined, line, "tokenizer changed the text: {runs:?}");
    runs
}

fn of(runs: &[(String, Token)], tok: Token) -> Vec<&str> {
    runs.iter()
        .filter(|(_, t)| *t == tok)
        .map(|(s, _)| s.as_str())
        .collect()
}

#[test]
fn rust_keywords_strings_and_comments() {
    let runs = assert_lossless(r#"    let name = "world"; // greet"#, "rust");
    assert_eq!(of(&runs, Token::Keyword), vec!["let"]);
    assert_eq!(of(&runs, Token::Str), vec![r#""world""#]);
    assert_eq!(of(&runs, Token::Comment), vec!["// greet"]);
}

#[test]
fn python_uses_hash_comments_and_its_own_words() {
    let runs = assert_lossless("def go(x):  # entry", "python");
    assert_eq!(of(&runs, Token::Keyword), vec!["def"]);
    assert_eq!(of(&runs, Token::Comment), vec!["# entry"]);
    // `//` is not a comment in python — it is integer division.
    let div = assert_lossless("y = x // 2", "python");
    assert!(of(&div, Token::Comment).is_empty(), "{div:?}");
}

/// A word is claimed only when the WHOLE identifier matches, or `iffy` and
/// `format` would light up as control flow.
#[test]
fn a_keyword_must_be_the_whole_word() {
    let runs = assert_lossless("iffy(letter, selfish)", "rust");
    assert!(of(&runs, Token::Keyword).is_empty(), "{runs:?}");
}

/// Precedence: whichever region opens first owns the rest of itself.
#[test]
fn a_comment_marker_inside_a_string_is_not_a_comment() {
    let runs = assert_lossless(r#"let u = "http://x.dev";"#, "rust");
    assert!(of(&runs, Token::Comment).is_empty(), "{runs:?}");
    assert_eq!(of(&runs, Token::Str), vec![r#""http://x.dev""#]);
}

#[test]
fn a_quote_inside_a_comment_does_not_open_a_string() {
    let runs = assert_lossless(r#"x(); // it's fine"#, "rust");
    assert!(of(&runs, Token::Str).is_empty(), "{runs:?}");
    assert_eq!(of(&runs, Token::Comment), vec!["// it's fine"]);
}

#[test]
fn an_escaped_quote_does_not_end_the_string() {
    let runs = assert_lossless(r#"p("a\"b", 1)"#, "rust");
    assert_eq!(of(&runs, Token::Str), vec![r#""a\"b""#]);
}

/// An unterminated string runs to end of line rather than swallowing the
/// tokenizer or losing the tail.
#[test]
fn an_unterminated_string_ends_at_the_line() {
    let runs = assert_lossless(r#"let s = "oops"#, "rust");
    assert_eq!(of(&runs, Token::Str), vec![r#""oops"#]);
}

#[test]
fn shell_and_go_get_their_own_keywords() {
    let sh = assert_lossless("if [ -f x ]; then echo hi; fi", "bash");
    assert!(of(&sh, Token::Keyword).contains(&"if"), "{sh:?}");
    assert!(of(&sh, Token::Keyword).contains(&"fi"), "{sh:?}");
    let go = assert_lossless("func main() { defer f() }", "go");
    assert!(of(&go, Token::Keyword).contains(&"func"), "{go:?}");
}

/// An unlabelled fence still tokenizes — strings and C-family comments are
/// right often enough to be worth it, and a wrong guess reads as plain code.
#[test]
fn an_unknown_language_still_finds_strings() {
    let runs = assert_lossless(r#"set x = "y" // note"#, "");
    assert_eq!(of(&runs, Token::Str), vec![r#""y""#]);
    assert_eq!(of(&runs, Token::Comment), vec!["// note"]);
}

#[test]
fn empty_and_whitespace_lines_are_safe() {
    assert!(tokenize("", "rust").is_empty());
    let runs = assert_lossless("    ", "rust");
    assert_eq!(of(&runs, Token::Plain), vec!["    "]);
}

/// Unicode must not be split mid-character or reordered.
#[test]
fn unicode_survives() {
    assert_lossless("let s = \"héllo → 世界\"; // ✓", "rust");
}

/// Neighbouring runs of one token collapse, so the renderer places fewer
/// spans than there are characters.
#[test]
fn adjacent_runs_of_the_same_token_merge() {
    let runs = assert_lossless("a + b + c", "rust");
    assert_eq!(runs.len(), 1, "{runs:?}");
}
