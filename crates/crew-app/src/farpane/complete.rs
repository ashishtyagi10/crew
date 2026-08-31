//! Pure completion engine for the Far command bar: which token the caret
//! sits in (`caret_token`), the ranked candidate list for that token
//! (`candidates`), and applying a chosen candidate back into the command
//! line (`apply`). Everything here takes `(text, cwd, binaries)` as
//! parameters and returns data — no globals, no I/O beyond the single
//! bounded `read_dir` a `Path`-kind lookup needs — so it's unit-testable
//! against tempdirs without touching a real `FarPane`.
use std::collections::HashSet;
use std::path::Path;

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

/// Which token the caret sits in and its text so far. The first
/// whitespace-separated word is `Command`; every later word — including
/// `cd`'s argument — is `Path`.
pub(crate) fn caret_token(text: &str) -> (TokenKind, &str) {
    let token_start = text.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
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
/// [`scan_path_binaries`]; directory listings are sorted here).
pub(crate) fn candidates(text: &str, cwd: &Path, binaries: &[String]) -> Vec<String> {
    let (kind, token) = caret_token(text);
    match kind {
        TokenKind::Command => command_candidates(token, binaries),
        TokenKind::Path => path_candidates(token, cwd),
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

/// Path-kind candidates: split `token` at its last `/` into the directory
/// part (kept literal — `~`/`$VAR` stay unexpanded in the returned string)
/// and the name prefix to match; read that one directory (expanded via
/// `pathexpand` against `cwd`) once, prefix-match its entries, and suffix
/// directories with `/`.
fn path_candidates(token: &str, cwd: &Path) -> Vec<String> {
    let (dir_part, name_prefix) = match token.rfind('/') {
        Some(i) => (&token[..=i], &token[i + 1..]),
        None => ("", token),
    };
    let dir = if dir_part.is_empty() {
        cwd.to_path_buf()
    } else {
        crate::pathexpand::expand_path(cwd, dir_part)
    };
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut entries: Vec<(String, bool)> = read
        .filter_map(|e| e.ok())
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (e.file_name().to_string_lossy().into_owned(), is_dir)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    rank_prefix(name_prefix, names.into_iter())
        .into_iter()
        .map(|name| {
            let is_dir = entries.iter().any(|(n, d)| *n == name && *d);
            format!("{dir_part}{name}{}", if is_dir { "/" } else { "" })
        })
        .collect()
}

/// Case-sensitive prefix matches first, then case-insensitive matches not
/// already included — each group in `items`'s given order, deduped.
fn rank_prefix<'a>(prefix: &str, items: impl Iterator<Item = &'a str>) -> Vec<String> {
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
    let token_start = text.rfind(char::is_whitespace).map(|i| i + 1).unwrap_or(0);
    format!("{}{}", &text[..token_start], candidate)
}

/// An in-progress Tab-completion cycle on `FarPane::complete`: the ranked
/// candidates, which one is currently applied, and the pre-cycle command
/// line text (`prefix`) so Esc can restore it.
pub(crate) struct CycleState {
    pub(crate) candidates: Vec<String>,
    pub(crate) i: usize,
    pub(crate) prefix: String,
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
