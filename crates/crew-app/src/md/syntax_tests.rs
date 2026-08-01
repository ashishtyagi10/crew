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

/// A ```diff fence colours by LINE: the marker at column zero claims the
/// whole line, and the `+++`/`---` file headers dim rather than shout.
#[test]
fn diff_fences_colour_by_line() {
    for lang in ["diff", "patch"] {
        assert_eq!(
            assert_lossless("+added line", lang),
            vec![("+added line".to_string(), Token::Added)],
        );
        assert_eq!(assert_lossless("-removed line", lang)[0].1, Token::Removed);
        assert_eq!(assert_lossless("@@ -1,2 +1,2 @@", lang)[0].1, Token::Hunk);
        assert_eq!(assert_lossless("+++ b/file.rs", lang)[0].1, Token::Comment);
        assert_eq!(assert_lossless("--- a/file.rs", lang)[0].1, Token::Comment);
    }
}

/// A rust fence whose line happens to start with `-` is arithmetic, not a
/// removal — only the diff/patch tags (or layout's untagged sniff) get the
/// line colouring.
#[test]
fn a_rust_fence_with_a_leading_minus_is_not_a_diff() {
    let runs = assert_lossless("-x - 1", "rust");
    assert!(runs.iter().all(|(_, t)| *t != Token::Removed), "{runs:?}");
}

/// The re-review's defect: the scan loop used to rebuild the remainder of the
/// line as a fresh `String` (`chars[i..].iter().collect()`) on every
/// position, just to ask whether a comment marker started there — O(L) work
/// and an O(L) allocation per character scanned, O(L²) overall. A line with
/// no comment marker or quote anywhere is the worst case: the scan runs to
/// the end with no early exit. 20 000 characters is far past where the old
/// code stopped being practical (it hit ~1s around 50 000 in release), so
/// this stands in for the one-giant-line minified files the viewer now
/// actually feeds `tokenize`. No wall-clock assertion here — a slow test is
/// not a failing one — but see the fix report for measured before/after
/// timings from a standalone replica of both loops.
#[test]
fn a_long_line_with_no_comment_or_string_is_still_lossless() {
    let line: String = "x".repeat(20_000);
    let runs = assert_lossless(&line, "js");
    // Not just lossless — a single 20 000-char plain run is exactly what a
    // linear scan with no comment/string hits should produce. A tokenizer
    // that silently fragmented the line (e.g. character-at-a-time runs from
    // a broken merge step) would still pass `assert_lossless` while doing far
    // more allocation than intended, so this pins the shape too.
    assert_eq!(runs.len(), 1, "{}", runs.len());
    assert_eq!(runs[0].1, Token::Plain);
}

/// The concrete scenario named in the review: a minified JSON/JS line is one
/// giant line packed with many short strings and no line breaks. This checks
/// the tokenizer stays both correct (lossless) and actually does its job
/// (finds the strings) at a size representative of that shape, not just that
/// it survives without panicking.
#[test]
fn a_long_minified_json_line_is_still_lossless_and_finds_every_string() {
    let mut line = String::from("{");
    for i in 0..5_000 {
        line.push_str(&format!("\"k{i}\":\"v{i}\","));
    }
    line.push('}');
    let runs = assert_lossless(&line, "json");
    let strings = of(&runs, Token::Str);
    // Two string tokens per iteration (the key and the value); a lexer that
    // regressed to treating the whole thing as one opaque Plain run (still
    // lossless!) would fail this while passing `assert_lossless`.
    assert_eq!(strings.len(), 10_000, "{}", strings.len());
}
