//! The word pass: on main the lexer knew keyword / string / comment and
//! nothing else, so `Vec`, `clamp(`, `42` and `#[derive]` were all `Plain`.
use crate::md::syntax::{tokenize, Token};

fn lossless(line: &str, lang: &str) -> Vec<(String, Token)> {
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
fn rust_types_calls_numbers_and_attributes() {
    let runs = lossless(
        "#[derive(Clone)] fn go(v: Vec<u8>) -> u8 { v.len() + 42 }",
        "rust",
    );
    assert_eq!(of(&runs, Token::Attr), vec!["#[derive(Clone)]"]);
    assert_eq!(of(&runs, Token::Keyword), vec!["fn"]);
    assert_eq!(of(&runs, Token::Func), vec!["go", "len"]);
    assert_eq!(of(&runs, Token::Type), vec!["Vec"]);
    assert_eq!(of(&runs, Token::Number), vec!["42"]);
}

/// A type wins over a call: `Some(` and `Ok(` are the type they name.
#[test]
fn a_capitalised_call_is_a_type() {
    let runs = lossless("Some(String::new())", "rust");
    assert_eq!(of(&runs, Token::Type), vec!["Some", "String"]);
    assert_eq!(of(&runs, Token::Func), vec!["new"]);
}

/// `Self`/`None`/`True` are keywords before they are capitalised.
#[test]
fn keywords_win_over_types() {
    assert!(of(&lossless("Self::new()", "rust"), Token::Type).is_empty());
    assert_eq!(
        of(&lossless("x = None", "python"), Token::Keyword),
        vec!["None"]
    );
}

/// A decimal point inside a number stays inside it; a dotted path does not.
#[test]
fn numbers_keep_their_decimal_point_and_suffix() {
    let runs = lossless("let s = 0.5 * 8usize + 0xff;", "rust");
    assert_eq!(of(&runs, Token::Number), vec!["0.5", "8usize", "0xff"]);
    let path = lossless("a.b(1.2.3)", "rust");
    assert_eq!(of(&path, Token::Number), vec!["1.2", "3"]);
    assert_eq!(of(&path, Token::Func), vec!["b"]);
}

/// Python and TypeScript decorators; a `@` with nothing after it is not one,
/// and an `@` inside an identifier (an email in a string is a string anyway)
/// is not one either.
#[test]
fn decorators_are_attributes_in_python_and_typescript() {
    let py = lossless("@app.route('/')", "python");
    assert_eq!(of(&py, Token::Attr), vec!["@app.route"]);
    assert_eq!(of(&py, Token::Str), vec!["'/'"]);
    let ts = lossless("@Component({}) class App {}", "ts");
    assert_eq!(of(&ts, Token::Attr), vec!["@Component"]);
    assert_eq!(of(&ts, Token::Type), vec!["App"]);
    assert!(of(&lossless("a @ b", "python"), Token::Attr).is_empty());
}

/// Rust attributes nest brackets and run to end of line when unclosed; an
/// inner attribute `#![…]` counts too. Shell's `#` is still a comment.
#[test]
fn rust_attributes_close_on_their_own_bracket() {
    let runs = lossless("#[cfg(all(a, b))] x = [1]", "rust");
    assert_eq!(of(&runs, Token::Attr), vec!["#[cfg(all(a, b))]"]);
    assert_eq!(of(&runs, Token::Number), vec!["1"]);
    assert_eq!(
        of(&lossless("#![allow(x)]", "rust"), Token::Attr),
        vec!["#![allow(x)]"]
    );
    assert_eq!(
        of(&lossless("#[derive(", "rust"), Token::Attr),
        vec!["#[derive("]
    );
    assert_eq!(of(&lossless("#[x]", "bash"), Token::Comment), vec!["#[x]"]);
}

/// Only the languages that spell types by case get them: a capitalised shell
/// variable and a word in an unlabelled fence stay plain.
#[test]
fn types_only_where_the_convention_holds() {
    assert!(of(&lossless("echo $HOME", "bash"), Token::Type).is_empty());
    assert!(of(&lossless("Hello world", ""), Token::Type).is_empty());
    assert_eq!(
        of(&lossless("func (s Server) Run()", "go"), Token::Type),
        vec!["Server", "Run"]
    );
}

/// The old three classes are untouched by the new ones: a string holding a
/// call, a comment holding a number, are still one run each.
#[test]
fn strings_and_comments_still_claim_their_region_whole() {
    let runs = lossless(r#"f("g(1)") // Vec 2"#, "rust");
    assert_eq!(of(&runs, Token::Func), vec!["f"]);
    assert_eq!(of(&runs, Token::Str), vec![r#""g(1)""#]);
    assert_eq!(of(&runs, Token::Comment), vec!["// Vec 2"]);
    assert!(of(&runs, Token::Number).is_empty());
}
