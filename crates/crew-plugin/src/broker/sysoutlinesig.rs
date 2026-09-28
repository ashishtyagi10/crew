//! What an outline row says about a definition: its head, one line, with the
//! parameters cut to their names.
//!
//! A row is there to be recognised and jumped to, and at ~5,600 bytes for the
//! whole outline every character of it is one another row cannot have.
//! `pub fn frame(env: &Envelope, intro: Option<&str>, …) -> String` names the
//! function no better than `pub fn frame(env, intro, …) -> String`, and the
//! types are one `sys:read_file` away, at the line the row gives.

#[cfg(test)]
#[path = "sysoutlinesig_tests.rs"]
mod tests;

/// Chars of one row's text; the rest becomes `…`.
pub(super) const ROW_CHARS: usize = 100;

/// Lines one definition's head may run over before the rest is let go: a
/// parameter list one to a line is long, and one longer is not a head.
const HEAD_LINES: usize = 12;

/// The head of the definition on `lines[0]`, trimmed and joined with the
/// lines it runs over while its brackets stay open, then cut before the first
/// of `stops` outside them (`{`, `;`, ` where `, …) from byte `from` on, the
/// keyword, so `pub(crate)` is never taken for a tuple struct's `(`.
pub(super) fn head(lines: &[String], from: usize, stops: &[&str]) -> String {
    let mut text = String::new();
    let mut open = 0i32;
    for line in lines.iter().take(HEAD_LINES) {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(line.trim());
        open += line
            .chars()
            .map(|ch| match ch {
                '(' | '[' => 1,
                ')' | ']' => -1,
                _ => 0,
            })
            .sum::<i32>();
        if open <= 0 {
            break;
        }
    }
    cut(&text, from, stops).trim_end().to_string()
}

/// `s` up to the first of `stops` at or after byte `from` outside `()` and
/// `[]`.
pub(super) fn cut<'a>(s: &'a str, from: usize, stops: &[&str]) -> &'a str {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        if i >= from && depth <= 0 && stops.iter().any(|st| s[i..].starts_with(st)) {
            return &s[..i];
        }
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
    }
    s
}

/// `sig` with the parameter list that opens at or after byte `from` cut to
/// its names (`(env: &Envelope, peers)` → `(env, peers)`), and clipped to a
/// row. Generics before the list are passed over, whatever `(` they hold.
pub(super) fn compact(sig: &str, from: usize) -> String {
    let Some(open) = params_open(sig, from) else {
        return clip(sig);
    };
    let Some(close) = matching(sig, open) else {
        return clip(sig);
    };
    let names: Vec<&str> = split_top(&sig[open + 1..close], ',')
        .into_iter()
        .map(param_name)
        .filter(|n| !n.is_empty())
        .collect();
    clip(&format!(
        "{}{}{}",
        &sig[..=open],
        names.join(", "),
        &sig[close..]
    ))
}

/// `s` whitespace-flattened and clipped to [`ROW_CHARS`].
pub(super) fn clip(s: &str) -> String {
    super::route::clip(s, ROW_CHARS)
}

/// A parameter's name: what comes before its type (`: T`) or its default
/// (`= 3`), and without `mut`. `self`, `&mut self` and `*args` have neither.
fn param_name(p: &str) -> &str {
    let p = p.trim();
    let p = p.strip_prefix("mut ").unwrap_or(p).trim_start();
    let b = p.as_bytes();
    let mut depth = 0i32;
    for (i, &ch) in b.iter().enumerate() {
        let prev = i.checked_sub(1).map(|k| b[k]);
        let next = b.get(i + 1).copied();
        match ch {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b'>' if !matches!(prev, Some(b'-' | b'=')) => depth -= 1,
            b':' if depth <= 0 && prev != Some(b':') && next != Some(b':') => {
                return p[..i].trim_end();
            }
            b'=' if depth <= 0 && !matches!(next, Some(b'=' | b'>')) => {
                return p[..i].trim_end();
            }
            _ => {}
        }
    }
    p
}

/// The byte of the `(` opening the parameter list: the first at or after
/// `from` that is not inside `<…>` generics.
fn params_open(sig: &str, from: usize) -> Option<usize> {
    let b = sig.as_bytes();
    let mut angle = 0i32;
    for i in from.min(b.len())..b.len() {
        match b[i] {
            b'<' => angle += 1,
            b'>' if i == 0 || !matches!(b[i - 1], b'-' | b'=') => angle -= 1,
            b'(' if angle <= 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// The `)` closing the `(` at `open`, or `None` when the head was cut first.
fn matching(sig: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in sig[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `s` split at each `sep` outside brackets of any kind.
fn split_top(s: &str, sep: char) -> Vec<&str> {
    let b = s.as_bytes();
    let (mut depth, mut from, mut parts) = (0i32, 0, Vec::new());
    for (i, ch) in s.char_indices() {
        let prev = i.checked_sub(1).map(|k| b[k]);
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '>' if !matches!(prev, Some(b'-' | b'=')) => depth -= 1,
            c if c == sep && depth <= 0 => {
                parts.push(&s[from..i]);
                from = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&s[from..]);
    parts
}
