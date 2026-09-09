//! The word pass of the fence lexer: re-splits every `Plain` run on word
//! boundaries and classifies each word — keyword, type, number, function
//! call — then glues neighbouring runs of one token back together so the
//! renderer places fewer, longer spans.
//!
//! What a word IS is decided from the word alone plus one character of
//! lookahead (the `(` that makes a call), never from state carried across
//! words: the lexer is line-wise and this pass keeps it that way.
use super::syntax::Token;

/// What the lexer knows about the language, gathered once per line.
pub(super) struct Words {
    pub keywords: &'static [&'static str],
    pub types: bool,
}

/// Classify one whole word. Precedence, highest first: keyword (`Self`,
/// `None` are keywords before they are capitalised), number, type, call —
/// so `Some(` reads as the type it names rather than the call it makes, and
/// `Ok(` matches `Some(` instead of one being yellow and the other blue.
fn classify(word: &str, next: Option<char>, w: &Words) -> Token {
    if w.keywords.contains(&word) {
        return Token::Keyword;
    }
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        return Token::Number;
    }
    if w.types && word.starts_with(|c: char| c.is_uppercase()) {
        return Token::Type;
    }
    if next == Some('(') {
        return Token::Func;
    }
    Token::Plain
}

/// Whether `c` continues `word`. Identifier characters always do; a `.` does
/// only inside a number (`3.14` is one literal, `a.b` is two words), only
/// when a digit follows it, and only once (`1.2.3` is a version, not a
/// float).
fn joins(word: &[char], c: char, next: Option<char>) -> bool {
    if c.is_alphanumeric() || c == '_' {
        return true;
    }
    c == '.'
        && word[0].is_ascii_digit()
        && !word.contains(&'.')
        && next.is_some_and(|n| n.is_ascii_digit())
}

/// Append `text` as `tok`, merging into the previous run when it is the same
/// token.
fn push(text: String, tok: Token, out: &mut Vec<(String, Token)>) {
    if text.is_empty() {
        return;
    }
    match out.last_mut() {
        Some((prev, t)) if *t == tok => prev.push_str(&text),
        _ => out.push((text, tok)),
    }
}

/// Re-split every `Plain` run on word boundaries so words can be claimed,
/// then glue neighbouring runs of the same token back together.
pub(super) fn merge_words(runs: Vec<(String, Token)>, w: &Words) -> Vec<(String, Token)> {
    let mut out: Vec<(String, Token)> = Vec::new();
    for (text, tok) in runs {
        if tok != Token::Plain {
            push(text, tok, &mut out);
            continue;
        }
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let first = chars[i];
            if !(first.is_alphanumeric() || first == '_') {
                push(first.to_string(), Token::Plain, &mut out);
                i += 1;
                continue;
            }
            let mut j = i + 1;
            while j < chars.len() && joins(&chars[i..j], chars[j], chars.get(j + 1).copied()) {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();
            let tok = classify(&word, chars.get(j).copied(), w);
            push(word, tok, &mut out);
            i = j;
        }
    }
    out
}

#[cfg(test)]
#[path = "syntaxword_tests.rs"]
mod tests;
