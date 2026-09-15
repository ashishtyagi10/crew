//! The instructions a repo already carries: `AGENTS.md` and `CLAUDE.md`.
//!
//! Every agentic tool that came before crew taught projects to write their
//! conventions down in a file at the root — Codex reads `AGENTS.md`, Claude
//! Code reads `CLAUDE.md`, and most repos that have one have it because a
//! human wrote it for a human. Crew ignored both and asked you to retype the
//! same rules as `#notes`, which is the one thing the file exists to prevent.
//!
//! So it reads them, with no command and no import step: found on the way up
//! from the working directory, folded in front of every task as PROJECT
//! INSTRUCTIONS, above your own standing memory (`super::memory`) — the repo
//! says how the repo is worked on, you say what you want, and the nearer
//! voice is yours.
use std::path::{Path, PathBuf};

/// The names, in the order a directory is searched. Both are read when both
/// exist: a repo with a `CLAUDE.md` and an `AGENTS.md` meant both.
pub(crate) const FILES: &[&str] = &["AGENTS.md", "CLAUDE.md"];

/// Chars of instructions carried in front of a task. Smaller than it sounds:
/// a 4 KB `AGENTS.md` is about 600 words of rules, and past that the file is
/// documentation, which the agent can read with a tool when it needs to.
pub(crate) const CAP: usize = 4096;

/// Directories walked up from the working directory. Stops at a repo root
/// (`.git`) whatever the count, so a deep monorepo path still reaches its
/// root and a home directory full of markdown never gets swept in.
const MAX_UP: usize = 8;

/// One file that was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Found {
    /// What to call it in a report — the path relative to where the search
    /// started, or just the name when it was found right there.
    pub name: String,
    pub text: String,
}

/// Every instruction file above `base`, FARTHEST first: the repo root's rules
/// come before a subdirectory's, so the nearer file has the last word.
pub(crate) fn find_at(base: &Path) -> Vec<Found> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut cur = base.to_path_buf();
    for _ in 0..MAX_UP {
        dirs.push(cur.clone());
        if cur.join(".git").exists() {
            break;
        }
        match cur.parent() {
            Some(p) if p != cur => cur = p.to_path_buf(),
            _ => break,
        }
    }
    let mut out: Vec<Found> = Vec::new();
    for dir in dirs.iter().rev() {
        for name in FILES {
            let path = dir.join(name);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let text = text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            out.push(Found {
                name: label(base, dir, name),
                text,
            });
        }
    }
    out
}

/// `AGENTS.md` when it is right here, `../AGENTS.md` when it is not — said
/// the way the user would have to type it.
fn label(base: &Path, dir: &Path, name: &str) -> String {
    match dir == base {
        true => name.to_string(),
        false => match base.strip_prefix(dir) {
            Ok(rel) => {
                let ups = rel.components().count();
                format!("{}{name}", "../".repeat(ups))
            }
            Err(_) => dir.join(name).to_string_lossy().to_string(),
        },
    }
}

/// The block a task carries, and the names it was built from — `None` when
/// the project has no instructions, in which case the task is untouched.
pub(crate) fn block_at(base: &Path) -> Option<(String, Vec<String>)> {
    let found = find_at(base);
    if found.is_empty() {
        return None;
    }
    let names: Vec<String> = found.iter().map(|f| f.name.clone()).collect();
    let share = CAP / found.len();
    let mut body = String::new();
    for f in &found {
        if !body.is_empty() {
            body.push_str("\n\n");
        }
        body.push_str(&format!("--- {} ---\n", f.name));
        body.push_str(&super::thread::clip_chars(&f.text, share));
    }
    Some((body, names))
}

/// The live call: the project dir every disk-backed broker module shares.
pub(crate) fn block() -> Option<(String, Vec<String>)> {
    block_at(&project_dir())
}

fn project_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// `/doctor`'s line: the mark and the detail.
pub(crate) fn doctor_line(names: &[String]) -> (char, String) {
    match names.is_empty() {
        true => (
            '\u{2013}',
            "none (an AGENTS.md or CLAUDE.md is read)".into(),
        ),
        false => (
            '\u{2713}',
            format!("{} — followed by every task", names.join(", ")),
        ),
    }
}

#[cfg(test)]
#[path = "agentsmd_tests.rs"]
mod tests;
