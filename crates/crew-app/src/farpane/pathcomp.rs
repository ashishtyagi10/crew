//! Path-kind candidates for the Far command bar: the entries of one
//! directory that match the caret word. Split from [`super::complete`] for
//! the line cap, along the line between ranking words and reading disks.
//!
//! What a shell's completion does, this does: `cd` only offers folders,
//! dotfiles stay hidden until the word starts with `.`, `..` and `~` finish
//! to `../` and `~/`, a symlink to a folder is a folder, and a name with a
//! space comes back backslash-escaped so it stays one word.
use std::path::Path;

use super::shellword::{escape, unescape};

/// Candidates for `token` (as typed, escapes intact): split it at its last
/// `/` into the directory part (kept literal — `~`/`$VAR` stay unexpanded
/// in the returned string) and the name prefix to match; read that one
/// directory once; suffix directories with `/`.
pub(crate) fn path_candidates(token: &str, cwd: &Path, dirs_only: bool) -> Vec<String> {
    if token == "~" {
        return vec!["~/".to_string()];
    }
    let (dir_part, name_part) = match token.rfind('/') {
        Some(i) => (&token[..=i], &token[i + 1..]),
        None => ("", token),
    };
    let name_prefix = unescape(name_part);
    let dir = if dir_part.is_empty() {
        cwd.to_path_buf()
    } else {
        crate::pathexpand::expand_path(cwd, &unescape(dir_part))
    };
    let mut entries = read_entries(&dir, dirs_only);
    let dotted = name_prefix.starts_with('.');
    entries.retain(|(n, _)| dotted || !n.starts_with('.'));
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    if dotted && dir.parent().is_some() {
        entries.insert(0, ("..".to_string(), true));
    }
    let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    super::complete::rank_prefix(&name_prefix, names.into_iter())
        .into_iter()
        .map(|name| {
            let is_dir = entries.iter().any(|(n, d)| *n == name && *d);
            format!(
                "{dir_part}{}{}",
                escape(&name),
                if is_dir { "/" } else { "" }
            )
        })
        .collect()
}

/// `(name, is_dir)` for each entry of `dir`, following symlinks so a link to
/// a folder completes with a `/` and descends. Folders only when `dirs_only`.
fn read_entries(dir: &Path, dirs_only: bool) -> Vec<(String, bool)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.filter_map(|e| e.ok())
        .map(|e| {
            let is_dir = e.path().is_dir();
            (e.file_name().to_string_lossy().into_owned(), is_dir)
        })
        .filter(|(_, is_dir)| *is_dir || !dirs_only)
        .collect()
}

#[cfg(test)]
#[path = "pathcomp_tests.rs"]
mod tests;
