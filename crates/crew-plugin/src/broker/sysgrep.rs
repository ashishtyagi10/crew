//! `sys:grep` and `sys:glob`: finding code without guessing its path.
//!
//! Measured live (2026-09-27): asked "which file draws the todo pane's
//! checkbox?", a worker called `lsp:definition` on `src/gui/todos.rs` — a
//! file it made up — then `list_dir` on a directory that does not exist, and
//! burned three of its tool rounds learning the tree one guess at a time. It
//! had `read_file` and `list_dir` and no way to SEARCH. These are the two
//! searches every coding agent leans on: content by regex, paths by pattern.
//!
//! Pure Rust (a regex over the files git or a directory walk lists — see
//! `sysgrepfiles`), so they need no `rg` on the machine and behave the same
//! on Windows. Binary files and files over 1 MB are passed over, and the
//! output is fitted to what the agent is shown (`sysgrepfit`), ending with a
//! count of what was left out and where all of it was saved (`spill`).
use std::path::Path;

use super::sysgrepfiles::files;
use super::sysgrepfit::{counted, Fit};

/// A line's text is clipped to this many chars.
const LINE_CAP: usize = 200;
/// Files larger than this are not searched (generated, vendored, data).
const FILE_CAP: u64 = 1024 * 1024;
/// Lines of `context` a side at most: at five a hit is eleven lines, and six
/// such hits fill what the agent is shown.
const MAX_CONTEXT: usize = 5;
/// What the closing line of a cut result asks of the agent.
const NARROW: &str = "narrow the pattern, or pass \"path\" to search a subdirectory";

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

/// `sys:grep {"pattern", "path"?, "glob"?, "ignore_case"?, "context"?}` —
/// matching lines as `path:line: text`, and with `context` the lines around
/// each as `path-line- text`, grouped the way `grep -C` groups them.
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
    let context = context_arg(v)?;
    let re = regex::RegexBuilder::new(pattern)
        .case_insensitive(fold)
        .build()
        .map_err(|e| format!("not a valid regex: {e}"))?;
    let root = Path::new(root);
    if !root.exists() {
        return Err(format!("no such path: {}", root.display()));
    }
    let mut fit = Fit::spilling("grep");
    for (rel, path) in files(root) {
        if glob.is_some_and(|g| !glob_match(g, &rel))
            || std::fs::metadata(&path).map_or(true, |m| !m.is_file() || m.len() > FILE_CAP)
        {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.iter().take(8192).any(|b| *b == 0) {
            continue; // binary
        }
        let text = String::from_utf8_lossy(&bytes);
        let lines: Vec<&str> = text.lines().collect();
        let hits: Vec<usize> = (0..lines.len())
            .filter(|&n| re.is_match(lines[n]))
            .collect();
        if hits.is_empty() {
            continue;
        }
        let after_another = !fit.is_empty();
        fit.file();
        if !fit.keeping() && !fit.recording() {
            fit.count(hits.len());
            continue;
        }
        for item in grouped(&rel, &lines, &hits, context, after_another) {
            fit.push(item);
        }
    }
    if fit.is_empty() {
        return Ok(format!("no match for /{pattern}/ under {}", root.display()));
    }
    Ok(fit.finish(|hits, files| {
        let (hits, files) = (counted(hits, "more hit"), counted(files, "file"));
        format!("\u{2026} {hits} in {files} left out \u{2014} {NARROW}")
    }))
}

/// The optional `context`: a line count, clamped to [`MAX_CONTEXT`]; a quoted
/// number is taken as one, and anything else is an error the agent can read.
fn context_arg(v: &serde_json::Value) -> Result<usize, String> {
    let n = match v.get("context") {
        None | Some(serde_json::Value::Null) => Some(0),
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        Some(serde_json::Value::String(s)) => s.trim().parse().ok(),
        Some(_) => None,
    };
    n.map(|n| n.min(MAX_CONTEXT as u64) as usize)
        .ok_or_else(|| format!("\u{201c}context\u{201d} is a number of lines, 0 to {MAX_CONTEXT}"))
}

/// One file's `hits` (0-based line numbers, ascending) as the items the fit
/// counts: one per hit, so a cut result stops at a whole one. Each item is
/// its hit line with the context before it not already shown and the context
/// after it up to the next hit; hits whose context meets share one group, and
/// a group after another opens with `--`. With no context, an item is the
/// hit line alone, as it always was.
fn grouped(
    rel: &str,
    lines: &[&str],
    hits: &[usize],
    context: usize,
    after_another: bool,
) -> Vec<String> {
    let shown = |n: usize| lines[n].trim().chars().take(LINE_CAP).collect::<String>();
    let mut next = 0; // the first line not yet shown
    let mut items = Vec::with_capacity(hits.len());
    for (i, &hit) in hits.iter().enumerate() {
        let from = hit.saturating_sub(context).max(next);
        let mut upto = (hit + context).min(lines.len() - 1);
        if let Some(&after) = hits.get(i + 1) {
            upto = upto.min(after - 1);
        }
        let opens = i == 0 || from > next;
        let mut item = Vec::new();
        if context > 0 && opens && (i > 0 || after_another) {
            item.push("--".to_string());
        }
        for n in from..=upto {
            item.push(match (n == hit, shown(n)) {
                (true, text) => format!("{rel}:{}: {text}", n + 1),
                (false, text) if text.is_empty() => format!("{rel}-{}-", n + 1),
                (false, text) => format!("{rel}-{}- {text}", n + 1),
            });
        }
        next = upto + 1;
        items.push(item.join("\n"));
    }
    items
}

/// `sys:glob {"pattern", "path"?}` — the files whose name (or relative path,
/// when the pattern has a `/`) matches.
pub(crate) fn glob(v: &serde_json::Value) -> Result<String, String> {
    let pattern = v
        .get("pattern")
        .and_then(|p| p.as_str())
        .ok_or("missing string argument \u{201c}pattern\u{201d}")?;
    let root = Path::new(v.get("path").and_then(|p| p.as_str()).unwrap_or("."));
    let mut fit = Fit::spilling("glob");
    for (rel, _) in files(root)
        .into_iter()
        .filter(|(rel, _)| glob_match(pattern, rel))
    {
        fit.file();
        fit.push(rel);
    }
    if fit.is_empty() {
        return Ok(format!(
            "no file matches {pattern} under {}",
            root.display()
        ));
    }
    Ok(fit.finish(|paths, _| {
        let paths = counted(paths, "more path");
        format!("\u{2026} {paths} left out \u{2014} {NARROW}")
    }))
}

#[cfg(test)]
#[path = "sysgrep_tests.rs"]
mod tests;
