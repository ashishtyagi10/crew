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
mod tests {
    use super::*;

    fn fixture(key: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("crew_far_complete_{key}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("src")).unwrap();
        std::fs::create_dir_all(base.join("srcdocs")).unwrap();
        std::fs::write(base.join("src/main.rs"), b"x").unwrap();
        std::fs::write(base.join("src/Main.txt"), b"x").unwrap();
        std::fs::write(base.join("readme.md"), b"x").unwrap();
        base
    }

    #[test]
    fn caret_token_splits_command_and_path_words() {
        assert_eq!(caret_token("ls"), (TokenKind::Command, "ls"));
        assert_eq!(caret_token("ls src/fa"), (TokenKind::Path, "src/fa"));
    }

    #[test]
    fn caret_token_trailing_space_starts_an_empty_token() {
        assert_eq!(caret_token("ls "), (TokenKind::Path, ""));
    }

    #[test]
    fn cd_argument_is_always_a_path_token() {
        assert_eq!(caret_token("cd sr"), (TokenKind::Path, "sr"));
    }

    #[test]
    fn path_candidates_list_the_tokens_parent_dir_with_dir_slash_suffix() {
        let base = fixture("pathlist");
        let cands = candidates("ls ", &base, &[]);
        assert!(cands.contains(&"src/".to_string()), "{cands:?}");
        assert!(cands.contains(&"srcdocs/".to_string()), "{cands:?}");
        assert!(cands.contains(&"readme.md".to_string()), "{cands:?}");
    }

    #[test]
    fn path_candidates_prefix_match_case_sensitive_then_insensitive() {
        let base = fixture("pathcase");
        // "M" matches "Main.txt" case-sensitively first, "main.rs" only
        // case-insensitively — case-sensitive matches must come first.
        let cands = candidates("ls src/M", &base, &[]);
        assert_eq!(
            cands,
            vec!["src/Main.txt".to_string(), "src/main.rs".to_string()]
        );
    }

    #[test]
    fn a_unique_prefix_yields_a_single_candidate() {
        let base = fixture("unique");
        let cands = candidates("cat read", &base, &[]);
        assert_eq!(cands, vec!["readme.md".to_string()]);
    }

    #[test]
    fn command_candidates_are_builtins_plus_binaries_prefix_matched() {
        let base = fixture("cmdlist");
        let bins = vec!["cargo".to_string(), "cat".to_string(), "ls".to_string()];
        let cands = candidates("ca", &base, &bins);
        assert_eq!(cands, vec!["cargo".to_string(), "cat".to_string()]);
    }

    #[test]
    fn cd_argument_completes_directories_in_context() {
        let base = fixture("cdpath");
        let cands = candidates("cd sr", &base, &[]);
        assert_eq!(cands, vec!["src/".to_string(), "srcdocs/".to_string()]);
    }

    #[test]
    fn apply_replaces_only_the_caret_token() {
        assert_eq!(apply("ls src/fa", "src/farpane/"), "ls src/farpane/");
        assert_eq!(apply("ca", "cargo"), "cargo");
        assert_eq!(apply("ls ", "readme.md"), "ls readme.md");
    }

    #[cfg(unix)]
    #[test]
    fn scan_path_binaries_finds_executables_only_sorted_and_deduped() {
        use std::os::unix::fs::PermissionsExt;
        let base = std::env::temp_dir().join("crew_far_complete_pathscan");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let exe = base.join("mytool");
        std::fs::write(&exe, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(base.join("notes.txt"), b"x").unwrap();
        // Duplicate PATH entry — the scan must dedupe across dirs too.
        let path_var = format!("{}:{}", base.display(), base.display());
        assert_eq!(scan_path_binaries(&path_var), vec!["mytool".to_string()]);
    }
}
