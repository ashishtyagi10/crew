//! Pure completion engine for the Far command bar: which token the caret
//! sits in (`caret_token`), the ranked candidate list for that token
//! (`candidates`), and applying a chosen candidate back into the command
//! line (`apply`). Everything here takes `(text, cwd, binaries)` as
//! parameters and returns data — no globals, no I/O beyond the single
//! bounded `read_dir` a `Path`-kind lookup needs (see [`super::pathcomp`])
//! — so it's unit-testable against tempdirs without touching a real
//! `FarPane`.
use std::collections::HashSet;
use std::path::Path;

use super::shellword::token_start;

/// Which token the caret sits in — completion always assumes the caret is at
/// end-of-line (the command bar is append/pop only today).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// The first whitespace-separated word: builtins + `$PATH` binaries.
    Command,
    /// Any later word (including `cd`'s argument): directory entries.
    Path,
}

/// A builtin the command bar understands directly (not a `$PATH` binary).
const BUILTINS: [&str; 1] = ["cd"];

/// Which token the caret sits in and its text so far (escapes intact). The
/// first whitespace-separated word is `Command`; every later word —
/// including `cd`'s argument — is `Path`. A backslash-escaped space does
/// not end a word: `cd My\ Folder` is two words, not three.
pub(crate) fn caret_token(text: &str) -> (TokenKind, &str) {
    let token_start = token_start(text);
    let token = &text[token_start..];
    let is_first_word = text[..token_start].trim().is_empty();
    let kind = if is_first_word {
        TokenKind::Command
    } else {
        TokenKind::Path
    };
    (kind, token)
}

/// Ranked candidates for the caret token in `text`: full replacement
/// strings for that token, ready for [`apply`]. Case-sensitive prefix
/// matches come first, then case-insensitive ones, each group in the input
/// order (`binaries` is expected pre-sorted, as produced by
/// [`scan_path_binaries`]; directory listings are sorted in `pathcomp`).
/// A `cd` argument only ever completes to a folder.
pub(crate) fn candidates(text: &str, cwd: &Path, binaries: &[String]) -> Vec<String> {
    let (kind, token) = caret_token(text);
    match kind {
        TokenKind::Command => command_candidates(token, binaries),
        TokenKind::Path => {
            let dirs_only = text.split_whitespace().next() == Some("cd");
            super::pathcomp::path_candidates(token, cwd, dirs_only)
        }
    }
}

fn command_candidates(prefix: &str, binaries: &[String]) -> Vec<String> {
    let pool: Vec<&str> = BUILTINS
        .iter()
        .copied()
        .chain(binaries.iter().map(String::as_str))
        .collect();
    rank_prefix(prefix, pool.into_iter())
}

/// Case-sensitive prefix matches first, then case-insensitive matches not
/// already included — each group in `items`'s given order, deduped.
pub(super) fn rank_prefix<'a>(prefix: &str, items: impl Iterator<Item = &'a str>) -> Vec<String> {
    let items: Vec<&str> = items.collect();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for s in &items {
        if s.starts_with(prefix) && seen.insert(*s) {
            out.push(s.to_string());
        }
    }
    if !prefix.is_empty() {
        let lower = prefix.to_lowercase();
        for s in &items {
            if !seen.contains(s) && s.to_lowercase().starts_with(&lower) {
                seen.insert(*s);
                out.push(s.to_string());
            }
        }
    }
    out
}

/// The new full command line after replacing the caret token with
/// `candidate` — reuses the same end-of-line token boundary as
/// [`caret_token`], so `candidate` must be the token's full replacement
/// text (a bare command name, or a path candidate from [`candidates`],
/// which already includes the token's directory part).
pub(crate) fn apply(text: &str, candidate: &str) -> String {
    format!("{}{}", &text[..token_start(text)], candidate)
}

/// An in-progress Tab-completion cycle on `FarPane::complete`: the ranked
/// candidates, which one is currently applied, and the pre-cycle command
/// line text (`prefix`) so Esc can restore it.
pub(crate) struct CycleState {
    pub(crate) candidates: Vec<String>,
    pub(crate) i: usize,
    pub(crate) prefix: String,
}

/// The strip shown after the caret while a cycle runs: how far through
/// the list the applied candidate is, then every candidate's leaf name so
/// the next Tab is a choice, not a guess. Capped so a wide directory
/// cannot push the bar off the row.
const HINT_CAP: usize = 72;

impl CycleState {
    pub(crate) fn hint(&self) -> String {
        let leaves = self.candidates.iter().map(|c| {
            let leaf = c.trim_end_matches('/').rsplit('/').next().unwrap_or(c);
            format!("{leaf}{}", if c.ends_with('/') { "/" } else { "" })
        });
        let mut out = format!("  {}/{} ", self.i + 1, self.candidates.len());
        for leaf in leaves {
            if out.chars().count() + leaf.chars().count() + 2 > HINT_CAP {
                out.push_str(" \u{2026}");
                break;
            }
            out.push_str("  ");
            out.push_str(&leaf);
        }
        out
    }
}

/// Read each directory in `path_var` (a `:`-joined `$PATH`-style string)
/// once, collecting executable file names; sorted and deduped. Missing or
/// unreadable directories are skipped, not fatal — this is the background
/// scan `FarPane`'s first Command-kind Tab kicks off on its own thread.
pub(crate) fn scan_path_binaries(path_var: &str) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for dir in std::env::split_paths(path_var) {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.filter_map(|e| e.ok()) {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            #[cfg(unix)]
            let executable =
                std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o111 != 0;
            #[cfg(not(unix))]
            let executable = true;
            if executable {
                names.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.into_iter().collect()
}

#[cfg(test)]
#[path = "complete_tests.rs"]
mod tests;
