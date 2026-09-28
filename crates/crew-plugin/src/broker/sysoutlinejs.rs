//! The outline's recognisers for JavaScript/TypeScript and Go, on the brace
//! walk the Rust one uses ([`walk`]).
//!
//! JS names its functions three ways that matter (`function f`, `const f =
//! (…) =>`, a method in a class) and exports most of them, so `export`,
//! `default` and friends are passed over the way `pub` is. A `const` is
//! listed when it holds a function or is exported: a module's other
//! constants are its working, and listing them buried the functions. Go's
//! definitions all sit at the top, a method with its receiver in front.
use super::sysoutline::Item;
use super::sysoutlinelang::{walk, Block};
use super::sysoutlinescan::{code_lines, Flavor};
use super::sysoutlinesig::{clip, compact, cut, head};

#[cfg(test)]
#[path = "sysoutlinejs_tests.rs"]
mod tests;

/// What a class body cannot start a member with, so a line the walk
/// misplaced into one is not listed as a method called `if`.
const NOT_NAMES: &[&str] = &[
    "if", "for", "while", "switch", "catch", "return", "function", "new", "else", "do", "try",
];

/// The JS/TS definitions in `src`.
pub(super) fn js(src: &str) -> Vec<Item> {
    walk(
        &code_lines(src, Flavor::CLike),
        &|lines, inside| match inside {
            Some(Block::Class) => member(lines),
            _ => top(lines),
        },
    )
}

/// The Go definitions in `src`.
pub(super) fn go(src: &str) -> Vec<Item> {
    walk(&code_lines(src, Flavor::CLike), &|lines, _| {
        let t = lines[0].trim();
        let named = |rest: &str| rest.starts_with(|c: char| c.is_alphabetic() || c == '_');
        match t.strip_prefix("type ") {
            _ if t.starts_with("func ") => Some((clip(&head(lines, 0, &["{"])), None)),
            Some(rest) if named(rest) => Some((clip(&head(lines, 0, &["{", "="])), None)),
            _ => None,
        }
    })
}

/// `t` past the words in `words` that lead it, and whether one was `export`.
fn past<'a>(t: &'a str, words: &[&str]) -> (&'a str, bool) {
    let (mut rest, mut exported) = (t, false);
    while let Some(w) = words.iter().find(|w| rest.starts_with(**w)) {
        exported |= *w == "export ";
        rest = rest[w.len()..].trim_start();
    }
    (rest, exported)
}

/// The identifier `rest` starts with.
fn ident(rest: &str) -> &str {
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '$' | '#')))
        .unwrap_or(rest.len());
    &rest[..end]
}

/// A definition at the top of a module or in a namespace.
fn top(lines: &[String]) -> Option<(String, Option<Block>)> {
    let full = lines[0].trim();
    let words = ["export ", "default ", "declare ", "abstract ", "async "];
    let (rest, exported) = past(full, &words);
    let at = full.len() - rest.len();
    let word = ident(rest);
    let after = &rest[word.len()..];
    let named = after.starts_with(' ') && !ident(after.trim_start()).is_empty();
    let row = |stops: &[&str]| clip(&head(lines, at, stops));
    Some(match word {
        "function" => (compact(&head(lines, at, &["{", ";"]), at), None),
        "class" => (row(&["{"]), Some(Block::Class)),
        "interface" | "enum" if named => (row(&["{"]), None),
        "namespace" | "module" if after.starts_with(' ') => (row(&["{"]), Some(Block::Scope)),
        "type" if named => (row(&["=", ";"]), None),
        "const" if after.trim_start().starts_with("enum ") => (row(&["{"]), None),
        "const" | "let" | "var" if named => return binding(lines, at, exported),
        _ => return None,
    })
}

/// `const f = (…) =>`, `const f = function`, or an exported constant.
fn binding(lines: &[String], at: usize, exported: bool) -> Option<(String, Option<Block>)> {
    let text = head(lines, at, &[]);
    let value = text[at..].split_once('=').map(|(_, v)| v.trim_start())?;
    let value = value.strip_prefix("async ").unwrap_or(value);
    if value.starts_with("function") {
        return Some((compact(cut(&text, at, &["{"]), at), None));
    }
    // `(a) =>`, `<T>(a: T) =>` or `a =>`, and not `foo(() => 1)`, which
    // holds an arrow without being one.
    let lead = ident(value);
    let arrow_first = value.starts_with(['(', '<'])
        || (!lead.is_empty() && value[lead.len()..].trim_start().starts_with("=>"));
    let arrow = cut(&text, at, &["=>"]);
    if arrow_first && arrow.len() < text.len() {
        return Some((format!("{} =>", compact(arrow.trim_end(), at)), None));
    }
    exported.then(|| (clip(cut(&text, at, &[":", "=", ";"]).trim_end()), None))
}

/// A class member: a method, or a property holding an arrow function.
fn member(lines: &[String]) -> Option<(String, Option<Block>)> {
    let full = lines[0].trim();
    let words = [
        "public ",
        "private ",
        "protected ",
        "static ",
        "readonly ",
        "async ",
        "override ",
        "abstract ",
        "declare ",
        "get ",
        "set ",
        "*",
    ];
    let (rest, _) = past(full, &words);
    let at = full.len() - rest.len();
    let name = ident(rest);
    if name.is_empty() || NOT_NAMES.contains(&name) {
        return None;
    }
    let after = rest[name.len()..].trim_start();
    let after = after.strip_prefix('?').unwrap_or(after);
    if after.starts_with(['(', '<']) {
        return Some((compact(&head(lines, at, &["{", ";"]), at), None));
    }
    let text = head(lines, at, &[]);
    let arrow = cut(&text, at, &["=>"]);
    (after.starts_with('=') && arrow.len() < text.len())
        .then(|| (format!("{} =>", compact(arrow.trim_end(), at)), None))
}
