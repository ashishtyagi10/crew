//! One shell word on the Far command line: where the caret's word starts
//! when spaces can be backslash-escaped, what a typed word means once its
//! escapes and quotes come off, and how to write a name back so the shell
//! (and the next Tab) read it as one word. Pure string functions.

/// Byte index where the caret's word begins: one past the last whitespace
/// that is not backslash-escaped, so `cd My\ Folder/` is one word.
pub(crate) fn token_start(text: &str) -> usize {
    let mut start = 0;
    let mut escaped = false;
    for (i, c) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c.is_whitespace() {
            start = i + c.len_utf8();
        }
    }
    start
}

/// The word as the filesystem sees it: a matching pair of `"` or `'`
/// around the whole word comes off, then every `\x` becomes `x`.
pub(crate) fn unescape(word: &str) -> String {
    let inner = ['"', '\'']
        .into_iter()
        .find_map(|q| {
            word.strip_prefix(q)
                .and_then(|w| w.strip_suffix(q))
                .filter(|_| word.len() >= 2)
        })
        .unwrap_or(word);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next().unwrap_or('\\')),
            c => out.push(c),
        }
    }
    out
}

/// Characters a POSIX shell would read as more than a name.
const SPECIAL: &str = " \t\"'\\&|;<>()$`*?[]#";

/// `name` written so `sh -c` reads it as one literal word: a backslash
/// before whitespace, quotes, and the shell's operators.
pub(crate) fn escape(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if SPECIAL.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "shellword_tests.rs"]
mod tests;
