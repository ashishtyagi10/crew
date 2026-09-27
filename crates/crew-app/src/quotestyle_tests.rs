//! What you typed comes back in curly double quotes — “like this”.
//!
//! The status line had four styles for the same job: `'term'` (search, theme),
//! `"pattern"` (watch, notify, /keys), `‘name’` (Far) and `“x”` (/clear,
//! /look, /tools). This keeps the straight ones from coming back around an
//! interpolated value. Far's `‘file’` names a file, not your words, and stays.

/// Files where a straight-quoted value is syntax, not prose: an `@"path"`
/// mention, an HTTP header, a line injected into an agent's prompt.
const SYNTAX: &[&str] = &["filedrop.rs", "voice/openai.rs", "askroute.rs"];

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `'{name}'` or `\"{name}\"` inside a string literal.
fn straight_quoted(line: &str) -> bool {
    let mut rest = line;
    while let Some(i) = rest.find('{') {
        let before = &rest[..i];
        let Some(j) = rest[i..].find('}') else { break };
        let inner = &rest[i + 1..i + j];
        let after = &rest[i + j + 1..];
        let ident =
            !inner.is_empty() && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if ident
            && ((before.ends_with('\'') && after.starts_with('\''))
                || (before.ends_with("\\\"") && after.starts_with("\\\"")))
        {
            return true;
        }
        rest = &rest[i + j + 1..];
    }
    false
}

#[test]
fn typed_values_are_quoted_curly() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    let mut bad = Vec::new();
    for p in files {
        let rel = p
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel.ends_with("_tests.rs") || SYNTAX.contains(&rel.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        for (n, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") && straight_quoted(line) {
                bad.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "use \\u{{201c}}…\\u{{201d}}:\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_scan_sees_both_straight_styles() {
    assert!(straight_quoted(r#"format!("no match for '{term}'")"#));
    assert!(straight_quoted(r#"format!("watching \"{p}\"")"#));
    assert!(!straight_quoted(
        r#"format!("no match for \u{201c}{term}\u{201d}")"#
    ));
    assert!(!straight_quoted(r#"format!("{a}'s {b}")"#));
}
