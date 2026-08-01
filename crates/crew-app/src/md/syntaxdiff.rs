//! Line classes for `diff`/`patch` code fences.
//!
//! A diff is coloured by LINE, not by token — the marker at column zero
//! decides the whole line's ink — so this sits beside the lexer in
//! `syntax.rs` rather than inside it. The mapping matches the viewer's diff
//! rung (`viewpane::lines::diff_lines`): added green, removed red, hunk
//! cyan — with file-header lines pushed back to comment ink so the changed
//! lines stay the loudest thing in the block.
use super::syntax::Token;

/// Fence tags that name a diff outright.
pub(super) fn is_diff_lang(lang: &str) -> bool {
    matches!(lang, "diff" | "patch")
}

/// Whether an UNTAGGED fence body reads as a diff — the same sniff family as
/// `viewpane::detect::by_content`. A `diff --git` opener is proof on its own;
/// otherwise a `@@ ` hunk header must appear alongside at least one actual
/// `+`/`-` change line (file headers excluded), so a bullet list, a signature
/// of dashes, or prose with a stray `@@` never trips it.
pub(super) fn looks_like_diff(lines: &[String]) -> bool {
    if lines.first().is_some_and(|l| l.starts_with("diff --git")) {
        return true;
    }
    let hunk = lines.iter().any(|l| l.starts_with("@@ "));
    let changed = lines.iter().any(|l| {
        (l.starts_with('+') && !l.starts_with("+++"))
            || (l.starts_with('-') && !l.starts_with("---"))
    });
    hunk && changed
}

/// A whole line as one run — diff colouring is line-granular. Empty lines
/// yield no runs, matching `tokenize`.
pub(super) fn line_runs(line: &str) -> Vec<(String, Token)> {
    if line.is_empty() {
        return Vec::new();
    }
    vec![(line.to_string(), line_token(line))]
}

/// The line's class from its leading marker. Order matters: `+++`/`---` are
/// file headers, not a triple addition/removal, so the header arm claims
/// them before the single-character arms can.
fn line_token(line: &str) -> Token {
    const HEADERS: [&str; 4] = ["+++", "---", "diff --git", "index "];
    if HEADERS.iter().any(|h| line.starts_with(h)) {
        return Token::Comment;
    }
    if line.starts_with("@@") {
        return Token::Hunk;
    }
    match line.chars().next() {
        Some('+') => Token::Added,
        Some('-') => Token::Removed,
        _ => Token::Plain,
    }
}

#[cfg(test)]
#[path = "syntaxdiff_tests.rs"]
mod tests;
