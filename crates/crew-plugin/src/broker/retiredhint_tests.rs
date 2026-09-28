//! No line crew writes may send the user to a command it has retired.
//!
//! Found live (2026-09-28): every pane's first task said "snapshot taken
//! before this task — /restore lists them, /restore <n> puts one back", and
//! every file-changing task ended "/restore puts them back". `/restore` left
//! the broker in 0.22.27, and in the app it now REOPENS SAVED PANES — so the
//! advice was not just stale, it named a different action. The retirement
//! table says what replaced each command; this holds the rest of the broker's
//! words to it.
use super::RETIRED;

/// Where a retired name is allowed: the table that explains the retirement,
/// and the dispatcher that routes the old spelling to that explanation.
const EXPLAINS: &[&str] = &["retired.rs", "commands.rs"];

/// `/name` at a word start — not the tail of a path like `.crew/memory.md`.
fn names_command(line: &str, name: &str) -> bool {
    let needle = format!("/{name}");
    line.match_indices(&needle).any(|(i, _)| {
        let before = line[..i].chars().next_back();
        let after = line[i + needle.len()..].chars().next();
        let starts = before.is_none_or(|c| c.is_whitespace() || "\"'(`\u{201c}".contains(c));
        let whole = after.is_none_or(|c| !c.is_alphanumeric() && c != '.' && c != '_' && c != '-');
        starts && whole
    })
}

fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_string_crew_writes_names_a_retired_command() {
    let mut files = Vec::new();
    sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let mut hits = Vec::new();
    for path in &files {
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        if file.ends_with("_tests.rs") || file == "tests.rs" || EXPLAINS.contains(&file.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(path).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") || !code.contains('"') {
                continue;
            }
            for (name, _) in RETIRED {
                if names_command(line, name) {
                    hits.push(format!("{file}:{} names /{name}: {}", n + 1, code));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "retired commands offered as advice:\n{}",
        hits.join("\n")
    );
}

#[test]
fn a_path_segment_is_not_a_command() {
    assert!(!names_command("\"remembered — .crew/memory.md\"", "memory"));
    assert!(!names_command("\"see ./.crew/mcp.json\"", "mcp"));
    assert!(names_command("\"— /restore lists them\"", "restore"));
    assert!(names_command("\"/restore <n> puts one back\"", "restore"));
    assert!(!names_command("\"/restored\"", "restore"));
}
