//! Filesystem path completion for the input bar: `cd` completes directories,
//! while `/dump` completes files and directories. Both finish the final
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

/// The commands whose argument is a filesystem path.
///
/// This started as `/dump` alone, and stayed that way while `/view`, `/md`
/// and `/batch` were added — so the three commands people type a path into
/// most often were the three with no completion at all, silently. A list, so
/// the parity test below can hold it against the palette's own descriptions:
/// a command that says `<path>` or `<file>` and is not here is a completion
/// that quietly never happens, which is exactly how this drifted.
///
/// `cd` is deliberately absent: it completes DIRECTORIES only, through
/// [`crate::suggest::dir_suggest`], and folding it in here would start
/// offering it files.
pub(crate) const PATH_COMMANDS: [&str; 4] = ["/dump", "/view", "/md", "/batch"];

/// Path completion for a `<command> <partial>` line (files and directories),
/// or `None` when `text` is not one of [`PATH_COMMANDS`].
pub(crate) fn path_suggest(text: &str, base: &Path) -> Option<String> {
    let arg = PATH_COMMANDS
        .iter()
        .find_map(|c| text.strip_prefix(c)?.strip_prefix(' '))?;
    complete_path(arg, base, true)
}

/// Resolve the directory portion of a path argument to a directory to list:
/// `~/` expands to `$HOME`, an absolute path is kept, otherwise it joins `base`.
pub(crate) fn expand(dir_part: &str, base: &Path) -> PathBuf {
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

    /// Per-test fixture dir: a SHARED path raced under parallel runs (one
    /// test's remove_dir_all deleted the tree mid-assertion in the other).
    fn fixture(name: &str) -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("crew_pathcomplete_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("alpha")).unwrap();
        std::fs::write(base.join("readme.md"), b"x").unwrap();
        base
    }

    #[test]
    fn completes_dirs_only_or_files_too() {
        let base = fixture("complete");
        // dirs-only: the directory matches (trailing slash), the file does not.
        assert_eq!(complete_path("al", &base, false).as_deref(), Some("pha/"));
        assert_eq!(complete_path("read", &base, false), None);
        // files_too: the file completes (no trailing slash).
        assert_eq!(complete_path("read", &base, true).as_deref(), Some("me.md"));
    }

    #[test]
    fn every_path_command_completes_paths() {
        let base = fixture("suggest");
        for cmd in super::PATH_COMMANDS {
            assert_eq!(
                path_suggest(&format!("{cmd} al"), &base).as_deref(),
                Some("pha/"),
                "{cmd} does not complete a directory"
            );
            assert_eq!(
                path_suggest(&format!("{cmd} read"), &base).as_deref(),
                Some("me.md"),
                "{cmd} does not complete a file"
            );
        }
        // Other commands and a trailing-slash partial complete nothing.
        assert_eq!(path_suggest("/run al", &base), None);
        assert_eq!(path_suggest("/dump alpha/", &base), None);
        // The space is part of the prefix: a command name that merely STARTS
        // with a path command's name is a different command.
        assert_eq!(path_suggest("/viewer al", &base), None);
    }

    /// The palette's descriptions and the completion list must agree. Each
    /// of `/view`, `/md` and `/batch` shipped as a command you type a path
    /// into with no completion at all, for as long as it had existed, because
    /// nothing compared the two lists.
    #[test]
    fn every_command_that_takes_a_path_completes_one() {
        for c in crate::cmddefs::commands() {
            let takes_path = c.desc.contains("<path>") || c.desc.contains("<file>");
            assert_eq!(
                takes_path,
                super::PATH_COMMANDS.contains(&c.name),
                "{} says {:?} but is {}in PATH_COMMANDS",
                c.name,
                c.desc,
                if takes_path { "not " } else { "" }
            );
        }
    }
}
