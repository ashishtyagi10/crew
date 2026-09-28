//! Source lines with their strings and comments taken out, for the outline's
//! brace languages (Rust, JS/TS, Go).
//!
//! The outline finds a definition by the word its line starts with and nests
//! it by the braces around it, and both lie when they sit in a string or a
//! comment: `// fn old() {` is not a function, and the `{` in it, counted,
//! would bury everything after it one block deeper. A parser would know
//! better; this knows just enough to blank literals and drop comments line by
//! line, carrying a string or block comment left open over to the next line.

#[cfg(test)]
#[path = "sysoutlinescan_tests.rs"]
mod tests;

/// How a language writes the literals the scan has to see past.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Flavor {
    /// `'x'` is a char and `'a` a lifetime, `r#"…"#` is raw, a `"…"` string
    /// may run over lines, and block comments nest.
    Rust,
    /// `'…'` is a string (a rune in Go), a backtick string runs over lines
    /// and the quoted ones end with theirs.
    CLike,
}

/// Where a line leaves off: in code, in a block comment `n` deep, or in a
/// string closed by `close` (and, for a raw string, that many `#`s).
#[derive(Clone, Copy)]
enum State {
    Code,
    Block(u32),
    Str { close: char, raw: Option<usize> },
}

/// Each line of `src` as code: comments dropped, and every string or char
/// literal reduced to its bare quotes (`""`), so the line still reads as the
/// kind of line it is and a `{` in it is never counted.
pub(super) fn code_lines(src: &str, flavor: Flavor) -> Vec<String> {
    let mut state = State::Code;
    src.lines()
        .map(|line| {
            let (code, next) = scan(line, state, flavor);
            state = match next {
                // A quoted JS or Go string cannot run on; one left open is a
                // regex or a misreading, and dropping it costs one line.
                State::Str { close, .. } if flavor == Flavor::CLike && close != '`' => State::Code,
                s => s,
            };
            code
        })
        .collect()
}

/// One line, from `state`: its code, and the state the next line starts in.
fn scan(line: &str, mut state: State, flavor: Flavor) -> (String, State) {
    let c: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < c.len() {
        let (ch, next) = (c[i], c.get(i + 1).copied());
        match state {
            State::Block(n) => {
                if ch == '*' && next == Some('/') {
                    state = match n {
                        1 => {
                            out.push(' ');
                            State::Code
                        }
                        _ => State::Block(n - 1),
                    };
                    i += 2;
                } else if flavor == Flavor::Rust && ch == '/' && next == Some('*') {
                    state = State::Block(n + 1);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            State::Str { close, raw } => {
                if ch == '\\' && raw.is_none() {
                    i += 2;
                } else if ch == close && hashes_at(&c, i + 1) >= raw.unwrap_or(0) {
                    out.push(close);
                    state = State::Code;
                    i += 1 + raw.unwrap_or(0);
                } else {
                    i += 1;
                }
            }
            State::Code => {
                if ch == '/' && next == Some('/') {
                    break;
                }
                if ch == '/' && next == Some('*') {
                    state = State::Block(1);
                    i += 2;
                    continue;
                }
                let quoted = ch == '"' || (flavor == Flavor::CLike && matches!(ch, '\'' | '`'));
                if quoted {
                    out.push(ch);
                    state = State::Str {
                        close: ch,
                        raw: None,
                    };
                    i += 1;
                    continue;
                }
                if flavor == Flavor::Rust {
                    if let Some((hashes, quote)) = raw_start(&c, i) {
                        out.push('"');
                        state = State::Str {
                            close: '"',
                            raw: Some(hashes),
                        };
                        i = quote + 1;
                        continue;
                    }
                    if ch == '\'' {
                        i = char_literal(&c, i, &mut out);
                        continue;
                    }
                }
                out.push(ch);
                i += 1;
            }
        }
    }
    (out, state)
}

/// `#`s at `at`, the ones that must follow a raw string's closing quote.
fn hashes_at(c: &[char], at: usize) -> usize {
    c[at.min(c.len())..]
        .iter()
        .take_while(|&&h| h == '#')
        .count()
}

/// A raw string opening at `i` (`r"`, `r#"`, `br##"`): its `#` count and
/// where its quote is. An `r` inside a name, or `r#type`, is not one.
fn raw_start(c: &[char], i: usize) -> Option<(usize, usize)> {
    let ident = |k: usize| c[k].is_alphanumeric() || c[k] == '_';
    let starts = match i {
        0 => true,
        _ if c[i - 1] == 'b' => i < 2 || !ident(i - 2),
        _ => !ident(i - 1),
    };
    if c[i] != 'r' || !starts {
        return None;
    }
    let hashes = hashes_at(c, i + 1);
    let quote = i + 1 + hashes;
    (c.get(quote) == Some(&'"')).then_some((hashes, quote))
}

/// A `'` in Rust: a char literal (`'x'`, `'\n'`, `'\u{7b}'`) is written as
/// `''` and skipped whole; anything else is a lifetime or a label and stays.
/// Returns where the scan goes on.
fn char_literal(c: &[char], i: usize, out: &mut String) -> usize {
    let end = match c.get(i + 1) {
        Some('\\') => c[(i + 3).min(c.len())..]
            .iter()
            .position(|&q| q == '\'')
            .map(|p| i + 3 + p),
        Some(_) if c.get(i + 2) == Some(&'\'') => Some(i + 2),
        _ => None,
    };
    match end {
        Some(e) => {
            out.push_str("''");
            e + 1
        }
        None => {
            out.push('\'');
            i + 1
        }
    }
}
