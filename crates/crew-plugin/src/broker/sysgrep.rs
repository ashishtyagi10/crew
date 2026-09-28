//! `sys:grep` and `sys:glob`: finding code without guessing its path.
//!
//! Measured live (2026-09-27): asked "which file draws the todo pane's
//! checkbox?", a worker called `lsp:definition` on `src/gui/todos.rs` — a
//! file it made up — then `list_dir` on a directory that does not exist, and
//! burned three of its tool rounds learning the tree one guess at a time. It
//! had `read_file` and `list_dir` and no way to SEARCH. These are the two
//! searches every coding agent leans on: content by regex, paths by pattern.
//!
//! Pure Rust (a directory walk and a regex), so they need no `rg` on the
//! machine and behave the same on Windows. The walk skips what is never the
//! answer — `.git`, build output, dependency trees, hidden directories and
//! binary files — and the output is capped, with a count of what was cut.
use std::path::Path;

/// Matching lines returned at most.
const MAX_HITS: usize = 150;
/// Paths returned at most by `glob`.
const MAX_PATHS: usize = 300;
/// A line's text is clipped to this many chars.
const LINE_CAP: usize = 200;
/// Files larger than this are not searched (generated, vendored, data).
const FILE_CAP: u64 = 1024 * 1024;
/// Directories never descended into.
const SKIP_DIRS: &[&str] = &["target", "node_modules", "dist", "build", "__pycache__"];

/// Whether the walk should enter/consider `e`.
fn visible(e: &walkdir::DirEntry) -> bool {
    let name = e.file_name().to_string_lossy();
    if e.depth() == 0 {
        return true;
    }
    if name.starts_with('.') {
        return false;
    }
    !(e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()))
}

/// `*` and `?` over a file NAME, or over the whole relative path when the
/// pattern holds a `/` (`**` matches any run of directories).
pub(crate) fn glob_match(pattern: &str, rel: &str) -> bool {
    let subject = if pattern.contains('/') {
        rel
    } else {
        rel.rsplit('/').next().unwrap_or(rel)
    };
    let re = regex::escape(pattern)
        .replace(r"\*\*/", "(?:.*/)?")
        .replace(r"\*\*", ".*")
        .replace(r"\*", "[^/]*")
        .replace(r"\?", "[^/]");
    regex::Regex::new(&format!("^{re}$")).is_ok_and(|r| r.is_match(subject))
}

fn rel_of(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `sys:grep {"pattern", "path"?, "glob"?, "ignore_case"?}` — matching lines
/// as `path:line: text`.
pub(crate) fn grep(v: &serde_json::Value) -> Result<String, String> {
    let pattern = v
        .get("pattern")
        .and_then(|p| p.as_str())
        .ok_or("missing string argument \u{201c}pattern\u{201d}")?;
    let root = v.get("path").and_then(|p| p.as_str()).unwrap_or(".");
    let glob = v.get("glob").and_then(|g| g.as_str());
    let fold = v
        .get("ignore_case")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(fold)
        .build()
        .map_err(|e| format!("not a valid regex: {e}"))?;
    let root = Path::new(root);
    if !root.exists() {
        return Err(format!("no such path: {}", root.display()));
    }
    let (mut hits, mut more) = (Vec::new(), 0usize);
    let walk = walkdir::WalkDir::new(root).sort_by_file_name().into_iter();
    for e in walk.filter_entry(visible).flatten() {
        if !e.file_type().is_file() || e.metadata().map_or(true, |m| m.len() > FILE_CAP) {
            continue;
        }
        let rel = rel_of(root, e.path());
        if glob.is_some_and(|g| !glob_match(g, &rel)) {
            continue;
        }
        let Ok(bytes) = std::fs::read(e.path()) else {
            continue;
        };
        if bytes.iter().take(8192).any(|b| *b == 0) {
            continue; // binary
        }
        let text = String::from_utf8_lossy(&bytes);
        for (n, line) in text.lines().enumerate() {
            if !re.is_match(line) {
                continue;
            }
            if hits.len() == MAX_HITS {
                more += 1;
                continue;
            }
            let shown: String = line.trim().chars().take(LINE_CAP).collect();
            hits.push(format!("{rel}:{}: {shown}", n + 1));
        }
    }
    if hits.is_empty() {
        return Ok(format!("no match for /{pattern}/ under {}", root.display()));
    }
    if more > 0 {
        hits.push(format!(
            "\u{2026} {more} more matches \u{2014} narrow the pattern, path or glob"
        ));
    }
    Ok(hits.join("\n"))
}

/// `sys:glob {"pattern", "path"?}` — the files whose name (or relative path,
/// when the pattern has a `/`) matches.
pub(crate) fn glob(v: &serde_json::Value) -> Result<String, String> {
    let pattern = v
        .get("pattern")
        .and_then(|p| p.as_str())
        .ok_or("missing string argument \u{201c}pattern\u{201d}")?;
    let root = Path::new(v.get("path").and_then(|p| p.as_str()).unwrap_or("."));
    let (mut paths, mut more) = (Vec::new(), 0usize);
    let walk = walkdir::WalkDir::new(root).sort_by_file_name().into_iter();
    for e in walk.filter_entry(visible).flatten() {
        let rel = rel_of(root, e.path());
        if !e.file_type().is_file() || !glob_match(pattern, &rel) {
            continue;
        }
        if paths.len() == MAX_PATHS {
            more += 1;
        } else {
            paths.push(rel);
        }
    }
    if paths.is_empty() {
        return Ok(format!(
            "no file matches {pattern} under {}",
            root.display()
        ));
    }
    if more > 0 {
        paths.push(format!("\u{2026} {more} more"));
    }
    Ok(paths.join("\n"))
}

#[cfg(test)]
#[path = "sysgrep_tests.rs"]
mod tests;
