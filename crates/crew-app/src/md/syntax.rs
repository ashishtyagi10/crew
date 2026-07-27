//! A small syntax tokenizer for fenced code blocks.
//!
//! Deliberately not syntect or tree-sitter. Neither is in the workspace, both
//! are large, and both want a grammar per language to earn their keep — while
//! what a chat transcript needs is the three distinctions that carry almost
//! all of the legibility: this is a comment, this is a string, this is a
//! keyword. The md engine above it is hand-rolled for the same reason.
//!
//! Scope, stated plainly so nobody mistakes it for a parser: it is a line-wise
//! lexer with no state carried between lines. A string that spans lines, or a
//! `/* … */` comment that does, is only highlighted on the line where it
//! opens. That is a real limitation and an acceptable one here — a chat reply
//! shows short excerpts, and the alternative is carrying block state through
//! wrapping and re-layout for a payoff nobody would notice.

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
}

/// Line-comment openers per language, longest first so `///` is not mistaken
/// for `/` and `#!` is not mistaken for `#`.
fn comment_starts(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" | "py" | "sh" | "bash" | "zsh" | "shell" | "ruby" | "rb" | "yaml" | "yml"
        | "toml" | "makefile" | "dockerfile" | "r" | "perl" => &["#"],
        "sql" | "lua" | "haskell" | "hs" => &["--"],
        "lisp" | "clojure" | "clj" => &[";"],
        "vim" => &["\""],
        // C-family and everything unrecognised: `//` is the safest guess, and
        // a language without it simply gets no comment colouring rather than
        // a wrong one.
        _ => &["//"],
    }
}

/// Keywords per language. Small on purpose — the control-flow and declaration
/// words are what the eye uses to find structure; a complete list of every
/// reserved word would colour half the line and distinguish nothing.
fn keywords(lang: &str) -> &'static [&'static str] {
    const RUST: &[&str] = &[
        "fn", "let", "mut", "const", "struct", "enum", "impl", "trait", "pub", "use", "mod",
        "match", "if", "else", "for", "while", "loop", "return", "self", "Self", "where", "async",
        "await", "move", "ref", "dyn", "as", "in", "break", "continue", "type", "static", "unsafe",
    ];
    const PY: &[&str] = &[
        "def", "class", "import", "from", "return", "if", "elif", "else", "for", "while", "try",
        "except", "finally", "with", "as", "lambda", "yield", "pass", "raise", "in", "not", "and",
        "or", "None", "True", "False", "async", "await", "global", "assert",
    ];
    const JS: &[&str] = &[
        "function",
        "const",
        "let",
        "var",
        "class",
        "extends",
        "return",
        "if",
        "else",
        "for",
        "while",
        "switch",
        "case",
        "break",
        "continue",
        "new",
        "this",
        "import",
        "export",
        "default",
        "async",
        "await",
        "try",
        "catch",
        "finally",
        "throw",
        "typeof",
        "interface",
        "type",
        "enum",
        "public",
        "private",
        "readonly",
        "null",
        "undefined",
    ];
    const GO: &[&str] = &[
        "func",
        "var",
        "const",
        "type",
        "struct",
        "interface",
        "package",
        "import",
        "return",
        "if",
        "else",
        "for",
        "range",
        "switch",
        "case",
        "defer",
        "go",
        "chan",
        "map",
        "nil",
    ];
    const SH: &[&str] = &[
        "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "case", "esac",
        "function", "return", "export", "local", "echo", "set",
    ];
    match lang {
        "rust" | "rs" => RUST,
        "python" | "py" => PY,
        "js" | "javascript" | "ts" | "typescript" | "tsx" | "jsx" => JS,
        "go" | "golang" => GO,
        "sh" | "bash" | "zsh" | "shell" | "console" => SH,
        // An unlabelled or unknown fence gets the C-family words, which
        // overlap heavily with most curly-brace languages. Wrong keywords are
        // cheap here: a missed word reads as plain code, which is the default.
        _ => JS,
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
    let comments = comment_starts(&lang);
    let words = keywords(&lang);
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(String, Token)> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    // Flush the pending Plain run, classifying it as a keyword when the whole
    // run is one. Word runs are accumulated whole, so `iffy` never matches
    // `if` — the check is on the complete identifier.
    let flush = |out: &mut Vec<(String, Token)>, buf: &mut String| {
        if buf.is_empty() {
            return;
        }
        out.push((std::mem::take(buf), Token::Plain));
    };
    while i < chars.len() {
        let rest: String = chars[i..].iter().collect();
        if comments.iter().any(|c| rest.starts_with(*c)) {
            flush(&mut out, &mut buf);
            out.push((rest, Token::Comment));
            return merge_words(out, words);
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
    merge_words(out, words)
}

/// Re-split every `Plain` run on word boundaries so keywords can be claimed,
/// then glue neighbouring runs of the same token back together — fewer, longer
/// spans mean fewer cells for the renderer to place.
fn merge_words(runs: Vec<(String, Token)>, words: &[&str]) -> Vec<(String, Token)> {
    let mut out: Vec<(String, Token)> = Vec::new();
    let push = |text: String, tok: Token, out: &mut Vec<(String, Token)>| {
        if text.is_empty() {
            return;
        }
        match out.last_mut() {
            Some((prev, t)) if *t == tok => prev.push_str(&text),
            _ => out.push((text, tok)),
        }
    };
    for (text, tok) in runs {
        if tok != Token::Plain {
            push(text, tok, &mut out);
            continue;
        }
        let mut word = String::new();
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                word.push(ch);
                continue;
            }
            let is_kw = words.contains(&word.as_str());
            push(
                std::mem::take(&mut word),
                if is_kw { Token::Keyword } else { Token::Plain },
                &mut out,
            );
            push(ch.to_string(), Token::Plain, &mut out);
        }
        let is_kw = words.contains(&word.as_str());
        push(
            word,
            if is_kw { Token::Keyword } else { Token::Plain },
            &mut out,
        );
    }
    out
}

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod tests;
