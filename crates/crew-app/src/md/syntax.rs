//! A small syntax tokenizer for fenced code blocks.
//!
//! Deliberately not syntect or tree-sitter. Neither is in the workspace, both
//! are large, and both want a grammar per language to earn their keep — while
//! what a chat transcript needs is the handful of distinctions that carry
//! almost all of the legibility: this is a comment, this is a string, this is
//! a keyword, a type, a call, a number. The md engine above it is hand-rolled
//! for the same reason.
//!
//! Scope, stated plainly so nobody mistakes it for a parser: it is a line-wise
//! lexer with no state carried between lines. A string that spans lines, or a
//! `/* … */` comment that does, is only highlighted on the line where it
//! opens. That is a real limitation and an acceptable one here — a chat reply
//! shows short excerpts, and the alternative is carrying block state through
//! wrapping and re-layout for a payoff nobody would notice.
//!
//! The language tables live in `syntaxlang`; the word pass (keyword / type /
//! number / call) in `syntaxword`. This file is the character scanner.
use super::syntaxlang::{self, AttrStyle};
use super::syntaxword::{merge_words, Words};

/// What a run of code text is, for colouring. `Plain` covers everything the
/// lexer does not claim — identifiers, operators, whitespace — and is the
/// overwhelming majority of any line.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum Token {
    #[default]
    Plain,
    Comment,
    Str,
    Keyword,
    /// A numeric literal, `.` included when digits sit on both sides.
    Number,
    /// A capitalised identifier in a language that spells types that way.
    Type,
    /// An identifier immediately followed by `(`.
    Func,
    /// A Rust `#[attribute]` or a Python/TS `@decorator`.
    Attr,
    /// Diff-only line classes (see `syntaxdiff`): an added line, a removed
    /// line, a `@@` hunk header. File-header lines reuse `Comment`.
    Added,
    Removed,
    Hunk,
}

/// Whether `pat` occurs at position `i` in `chars`, compared char by char
/// without materializing the remainder of the line. `tokenize` used to build
/// `chars[i..].iter().collect::<String>()` on every position just to call
/// `starts_with` on it — an O(L) allocation per character, O(L²) overall for
/// a line of length L. Comment markers are at most a couple of characters
/// (see `comment_starts`), so this check is O(1) amortized per position.
fn matches_at(chars: &[char], i: usize, pat: &str) -> bool {
    pat.chars()
        .enumerate()
        .all(|(k, pc)| chars.get(i + k) == Some(&pc))
}

/// The end (exclusive) of an attribute opening at `i`, or `None` when no
/// attribute opens there. A bracket attribute runs to the `]` that closes its
/// own `[`, nesting counted, or to end of line; a decorator runs through its
/// dotted name (`@app.route`). A `@` with no name after it is not one.
fn attr_end(chars: &[char], i: usize, style: AttrStyle) -> Option<usize> {
    match style {
        AttrStyle::Bracket => {
            let open = if matches_at(chars, i, "#[") {
                i + 1
            } else if matches_at(chars, i, "#![") {
                i + 2
            } else {
                return None;
            };
            let mut depth = 0usize;
            for (j, &c) in chars.iter().enumerate().skip(open) {
                match c {
                    '[' => depth += 1,
                    ']' if depth == 1 => return Some(j + 1),
                    ']' => depth -= 1,
                    _ => {}
                }
            }
            Some(chars.len())
        }
        AttrStyle::Decorator => {
            if chars[i] != '@' || i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
                return None;
            }
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '.')
            {
                j += 1;
            }
            (j > i + 1).then_some(j)
        }
        AttrStyle::None => None,
    }
}

/// Split one line of code into `(text, token)` runs, left to right, covering
/// every character exactly once.
///
/// Order matters: a comment opener inside a string is not a comment, and a
/// quote inside a comment does not open a string, so whichever starts first
/// claims the rest of its region.
pub(crate) fn tokenize(line: &str, lang: &str) -> Vec<(String, Token)> {
    let lang = lang.to_ascii_lowercase();
    // A diff is coloured by LINE, not by token — hand the whole line to the
    // classifier instead of lexing it.
    if super::syntaxdiff::is_diff_lang(&lang) {
        return super::syntaxdiff::line_runs(line);
    }
    let comments = syntaxlang::comment_starts(&lang);
    let attrs = syntaxlang::attr_style(&lang);
    let words = Words {
        keywords: syntaxlang::keywords(&lang),
        types: syntaxlang::has_types(&lang),
    };
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(String, Token)> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    // Flush the pending Plain run; `merge_words` classifies its words after.
    let flush = |out: &mut Vec<(String, Token)>, buf: &mut String| {
        if !buf.is_empty() {
            out.push((std::mem::take(buf), Token::Plain));
        }
    };
    while i < chars.len() {
        if comments.iter().any(|c| matches_at(&chars, i, c)) {
            flush(&mut out, &mut buf);
            // The comment claims the rest of the line, so this is the one
            // place `chars[i..]` is materialized — once, not once per
            // position scanned to find it.
            let rest: String = chars[i..].iter().collect();
            out.push((rest, Token::Comment));
            return merge_words(out, &words);
        }
        if let Some(end) = attr_end(&chars, i, attrs) {
            flush(&mut out, &mut buf);
            out.push((chars[i..end].iter().collect(), Token::Attr));
            i = end;
            continue;
        }
        let c = chars[i];
        if c == '"' || c == '\'' || c == '`' {
            flush(&mut out, &mut buf);
            let mut s = String::from(c);
            let mut j = i + 1;
            while j < chars.len() {
                s.push(chars[j]);
                // A backslash escapes the next character, so `"a\"b"` is one
                // string rather than two.
                if chars[j] == '\\' && j + 1 < chars.len() {
                    s.push(chars[j + 1]);
                    j += 2;
                    continue;
                }
                if chars[j] == c {
                    j += 1;
                    break;
                }
                j += 1;
            }
            out.push((s, Token::Str));
            i = j;
            continue;
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut out, &mut buf);
    merge_words(out, &words)
}

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod tests;
