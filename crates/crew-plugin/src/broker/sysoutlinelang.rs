//! The outline's recogniser for Rust, and the brace walk it shares with the
//! JS/TS and Go ones.
//!
//! Line-based on purpose: a definition is a line that starts with its
//! keyword, once `pub`, `async`, `unsafe` and the like are passed over, and it
//! is listed only where definitions live: at the top of the file or directly
//! inside an `impl`, `trait` or `mod` block. A `fn` in a function body is a
//! detail of that function, not a place to jump to. Strings and comments are
//! blanked first ([`code_lines`]), so neither a doc comment that says `fn`
//! nor a `'{'` moves anything.
use super::sysoutline::Item;
use super::sysoutlinescan::{code_lines, Flavor};
use super::sysoutlinesig::{compact, head};

#[cfg(test)]
#[path = "sysoutlinelang_tests.rs"]
mod tests;

/// A definition whose own definitions are listed under it, by the kind of
/// block it is: a class's members are found differently from a module's.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Block {
    Scope,
    Class,
}

/// A recogniser, asked about a line that stands where a definition may, with
/// the lines from there on and the block it is directly inside (`None` at the
/// top): the row's text, and the kind of block it opens, if it is one.
pub(super) type Recognise<'a> =
    &'a dyn Fn(&[String], Option<Block>) -> Option<(String, Option<Block>)>;

/// The definitions of a brace language's `code` lines, nested by the braces
/// of the blocks [`Recognise`] said are blocks. Anything deeper (a body, a
/// struct's fields, a block nobody named) is passed over whole.
pub(super) fn walk(code: &[String], recognise: Recognise) -> Vec<Item> {
    let mut items = Vec::new();
    // Each listed block we are inside: the depth just inside its `{`.
    let mut open: Vec<(usize, Block)> = Vec::new();
    // A block named but not yet opened: the depth its `{` will be met at.
    let mut pending: Option<(usize, Block)> = None;
    let mut depth = 0usize;
    for (i, line) in code.iter().enumerate() {
        if open.last().map_or(0, |o| o.0) == depth {
            if let Some((text, block)) = recognise(&code[i..], open.last().map(|o| o.1)) {
                pending = block.map(|b| (depth, b));
                items.push(Item::at(i, open.len(), text));
            }
        }
        for ch in line.chars() {
            match ch {
                '{' => {
                    if let Some((at, block)) = pending.filter(|p| p.0 == depth) {
                        open.push((at + 1, block));
                        pending = None;
                    }
                    depth += 1;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    while open.last().is_some_and(|o| o.0 > depth) {
                        open.pop();
                    }
                }
                ';' if pending.is_some_and(|p| p.0 == depth) => pending = None,
                _ => {}
            }
        }
    }
    items
}

/// The Rust definitions in `src`.
pub(super) fn rust(src: &str) -> Vec<Item> {
    walk(&code_lines(src, Flavor::Rust), &|lines, _| item(lines))
}

/// Words that may come before a definition's keyword. `const` is one only
/// before `fn` (`const fn`, `const unsafe fn`).
fn modifier(t: &str) -> Option<&str> {
    for word in ["async ", "unsafe ", "default ", "extern \"\" ", "extern "] {
        if let Some(rest) = t.strip_prefix(word) {
            return Some(rest);
        }
    }
    if let Some(rest) = t.strip_prefix("pub") {
        if rest.starts_with(' ') {
            return Some(rest);
        }
        if rest.starts_with('(') {
            return rest.find(')').map(|close| &rest[close + 1..]);
        }
    }
    let rest = t.strip_prefix("const ")?;
    let after = rest.trim_start();
    ["fn ", "unsafe ", "async ", "extern "]
        .iter()
        .any(|w| after.starts_with(w))
        .then_some(rest)
}

/// The row for the Rust definition on `lines[0]`, if it is one, and whether
/// it is a block (`impl`, `trait`, `mod`) whose definitions go under it.
fn item(lines: &[String]) -> Option<(String, Option<Block>)> {
    let full = lines[0].trim();
    let mut rest = full;
    while let Some(r) = modifier(rest) {
        rest = r.trim_start();
    }
    let at = full.len() - rest.len();
    let word = rest
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '!'))
        .next()?;
    // A name follows the keyword: not `_` (`const _: () = …`), and not
    // nothing (`impl` alone at a line's end).
    let named = |kw: &str| {
        let name = rest[kw.len()..].trim_start();
        let first = name
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next();
        !matches!(first, None | Some("" | "_"))
    };
    let row = |stops: &[&str]| head(lines, at, stops);
    let scope = Some(Block::Scope);
    Some(match word {
        "fn" if named("fn") => (compact(&row(&["{", ";", " where "]), at), None),
        "struct" | "enum" | "union" if named(word) => (row(&["{", "(", ";", " where "]), None),
        "trait" if named("trait") => (row(&["{", " where "]), scope),
        "mod" if named("mod") => (row(&["{", ";"]), scope),
        "impl" if rest[4..].starts_with([' ', '<']) => (row(&["{", " where "]), scope),
        "const" | "static" if named(word) => (row(&[":", "=", ";"]), None),
        "type" if named("type") => (row(&["=", ";", " where "]), None),
        "macro_rules!" if named("macro_rules!") => (row(&["{", "(", "["]), None),
        _ => return None,
    })
}
