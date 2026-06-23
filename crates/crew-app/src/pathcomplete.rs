//! Filesystem path completion for the input bar: `cd` completes directories,
//! while `/edit`/`/open` complete files and directories. Both finish the final
//! path component against a base directory, returning the ghost suffix.
use std::path::{Path, PathBuf};

/// Completion suffix for partial path `arg` resolved against `base`. With
/// `files_too` false only directories match (for `cd`); otherwise files match
/// too. Directory matches gain a trailing `/`. `None` when the partial is empty,
/// already ends in `/`, or nothing matches.
pub(crate) fn complete_path(arg: &str, base: &Path, files_too: bool) -> Option<String> {
    if arg.is_empty() || arg.ends_with('/') {
        return None;
    }
    let (dir_part, leaf) = match arg.rfind('/') {
        Some(i) => (&arg[..=i], &arg[i + 1..]),
        None => ("", arg),
    };
    if leaf.is_empty() {
        return None;
    }
    let mut matches: Vec<(String, bool)> = std::fs::read_dir(expand(dir_part, base))
        .ok()?
        .flatten()
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            (name, e.path().is_dir())
        })
        .filter(|(n, is_dir)| (files_too || *is_dir) && n.starts_with(leaf) && n != leaf)
        .collect();
    matches.sort();
    matches.into_iter().next().map(|(n, is_dir)| {
        let suffix = n[leaf.len()..].to_string();
        if is_dir {
            format!("{suffix}/")
        } else {
            suffix
        }
    })
}

/// Path completion for an `/edit <partial>` or `/open <partial>` line (files and
/// directories), or `None` when `text` isn't one of those commands.
pub(crate) fn path_suggest(text: &str, base: &Path) -> Option<String> {
    let arg = text
        .strip_prefix("/edit ")
        .or_else(|| text.strip_prefix("/open "))?;
    complete_path(arg, base, true)
}

/// Resolve the directory portion of a path argument to a directory to list:
/// `~/` expands to `$HOME`, an absolute path is kept, otherwise it joins `base`.
fn expand(dir_part: &str, base: &Path) -> PathBuf {
    if dir_part.is_empty() {
        return base.to_path_buf();
    }
    if let Some(rest) = dir_part.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    let p = Path::new(dir_part);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::{complete_path, path_suggest};

    fn fixture() -> std::path::PathBuf {
        let base = std::env::temp_dir().join("crew_pathcomplete_test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("alpha")).unwrap();
        std::fs::write(base.join("readme.md"), b"x").unwrap();
        base
    }

    #[test]
    fn completes_dirs_only_or_files_too() {
        let base = fixture();
        // dirs-only: the directory matches (trailing slash), the file does not.
        assert_eq!(complete_path("al", &base, false).as_deref(), Some("pha/"));
        assert_eq!(complete_path("read", &base, false), None);
        // files_too: the file completes (no trailing slash).
        assert_eq!(complete_path("read", &base, true).as_deref(), Some("me.md"));
    }

    #[test]
    fn path_suggest_only_for_edit_and_open() {
        let base = fixture();
        assert_eq!(path_suggest("/edit al", &base).as_deref(), Some("pha/"));
        assert_eq!(path_suggest("/open read", &base).as_deref(), Some("me.md"));
        // other commands and a trailing-slash partial complete nothing.
        assert_eq!(path_suggest("/run al", &base), None);
        assert_eq!(path_suggest("/edit alpha/", &base), None);
    }
}
