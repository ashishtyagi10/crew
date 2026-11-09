//! The call sites `/review` reads alongside the diff. A diff shows the
//! definition that changed but never the code that depends on it, so a
//! reviewer given the diff alone cannot see the caller still passing the old
//! arity — the bug a staff engineer finds by grepping before approving. This
//! does that grep: the names a diff defines or edits inside, each looked up
//! with a bounded `git grep -w`, definitions and imports dropped.
use std::path::Path;
use std::process::Command;

use crew_hive::childproc::no_console_window;

/// Names looked up per review. A wide refactor touches dozens; the first few
/// (signature changes come first) carry the risk, and each one is a process.
const MAX_SYMBOLS: usize = 6;
/// Call sites in the prompt overall, and per name so one popular helper
/// cannot crowd out the rest.
const MAX_CALLERS: usize = 20;
const PER_SYMBOL: usize = 8;
/// A call-site line is context, not code to review: clip it.
const LINE_CAP: usize = 100;

/// Keywords that introduce a definition, across the languages crew edits.
const DEF_KEYWORDS: &[&str] = &[
    "fn", "def", "function", "func", "struct", "enum", "trait", "class",
];

/// Names every codebase calls everywhere (trait methods, entry points): a
/// grep for them returns noise, not the dependents of this change.
const COMMON: &[&str] = &[
    "new",
    "main",
    "default",
    "from",
    "into",
    "fmt",
    "drop",
    "clone",
    "eq",
    "hash",
    "cmp",
    "next",
    "len",
    "is_empty",
    "as_ref",
    "deref",
    "to_string",
    "build",
    "init",
    "setup",
    "tests",
];

/// The names `diff` defines on a changed line (signature edits, first), then
/// the names of functions whose bodies changed (the hunk header's context).
pub(crate) fn changed_symbols(diff: &str) -> Vec<String> {
    let mut changed = Vec::new();
    let mut enclosing = Vec::new();
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@") {
            if let Some((_, ctx)) = rest.split_once("@@") {
                enclosing.extend(defined_name(ctx));
            }
        } else if line.starts_with('+') || line.starts_with('-') {
            changed.extend(defined_name(&line[1..]));
        }
    }
    let mut out: Vec<String> = Vec::new();
    for name in changed.into_iter().chain(enclosing) {
        let keep = name.len() >= 3
            && !name.starts_with("test_")
            && !COMMON.contains(&name.as_str())
            && !out.contains(&name);
        if keep && out.len() < MAX_SYMBOLS {
            out.push(name);
        }
    }
    out
}

/// The identifier after a definition keyword on `line`, if any.
fn defined_name(line: &str) -> Option<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    words.windows(2).find_map(|w| {
        if !DEF_KEYWORDS.contains(&w[0]) {
            return None;
        }
        let name: String = w[1]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        (!name.is_empty()).then_some(name)
    })
}

/// True for a grep hit that is not a call: the definition itself, an import,
/// or a comment.
fn not_a_call(text: &str, name: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("use ")
        || t.starts_with("import ")
        || t.starts_with("from ")
        || t.starts_with("//")
        || t.starts_with('*')
        || defined_name(t).as_deref() == Some(name)
}

/// Tests call everything; a production caller is the one that breaks users.
pub(crate) fn is_test_path(path: &str) -> bool {
    path.contains("_tests.") || path.contains("/tests/") || path.contains("_test.")
}

/// [`call_sites`], cut to the few a review prompt has room for.
fn grep_callers(dir: &Path, name: &str) -> Vec<(String, String, String)> {
    let mut hits = call_sites(dir, name);
    hits.truncate(PER_SYMBOL);
    hits
}

/// `git grep -n -w` for `name` in `dir`'s tree (untracked files too, ignored
/// ones not): EVERY `(path, line, text)` hit that is a call, production
/// first — the receipt counts them (`receiptreach`), so none is cut here.
pub(crate) fn call_sites(dir: &Path, name: &str) -> Vec<(String, String, String)> {
    let out = no_console_window(&mut Command::new("git"))
        .current_dir(dir)
        .args(["grep", "-n", "-w", "-I", "--untracked", "-e", name])
        .output();
    let Ok(out) = out else { return Vec::new() };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut hits: Vec<(String, String, String)> = text
        .lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, ':');
            let (p, n, t) = (parts.next()?, parts.next()?, parts.next()?);
            (!not_a_call(t, name)).then(|| (p.to_string(), n.to_string(), t.to_string()))
        })
        .collect();
    hits.sort_by_key(|(p, _, _)| is_test_path(p));
    hits
}

/// The CALLERS block for the review prompt, or "" when the diff changes no
/// named definition or nothing calls what it changes.
pub(crate) fn section(dir: &Path, diff: &str) -> String {
    let mut lines = Vec::new();
    for name in changed_symbols(diff) {
        for (path, n, text) in grep_callers(dir, &name) {
            if lines.len() == MAX_CALLERS {
                break;
            }
            let t: String = text.trim().chars().take(LINE_CAP).collect();
            lines.push(format!("{name} — {path}:{n}: {t}"));
        }
    }
    if lines.is_empty() {
        return String::new();
    }
    format!(
        "CALLERS (call sites of what the diff changed, from `git grep -w` on \
         the tree; some may already be updated by the diff). Check each still \
         fits the changed definition; a caller left behind is a finding — cite \
         it as file:line:\n{}",
        lines.join("\n")
    )
}

#[cfg(test)]
#[path = "reviewcallers_tests.rs"]
mod tests;
