//! Is this line a runnable command? Powers the input bar's smart routing:
//! the first word must resolve to a real executable (hydrated login-shell
//! PATH, explicit path) or a shell builtin before crew will spawn a pane
//! for it — so typos hint instead of littering dead panes.
use std::path::{Path, PathBuf};

/// What the first word of an input line turned out to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Resolves to an executable (name is the bare first word).
    Executable(String),
    /// A shell builtin that would be pointless in a throwaway pane.
    Builtin(String),
    /// Not something we can run.
    No,
}

/// State-mutating builtins: running them in a fresh pane silently does
/// nothing useful (the pane's shell exits with the state). `cd` is handled
/// earlier in submit_input, `echo`/`printf` etc. exist as real binaries.
const BUILTINS: &[&str] = &[
    "export", "set", "unset", "source", ".", "alias", "unalias", "eval",
];

/// The command word of `line`: the first whitespace token after skipping
/// leading `VAR=value` assignments, with surrounding quotes stripped.
pub(crate) fn first_word(line: &str) -> Option<String> {
    let word = line
        .split_whitespace()
        .find(|t| !is_assignment(t))?
        .trim_matches(|c| c == '"' || c == '\'');
    (!word.is_empty()).then(|| word.to_string())
}

/// `FOO=bar` (an env prefix), as opposed to a command word.
fn is_assignment(token: &str) -> bool {
    match token.split_once('=') {
        Some((name, _)) => {
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        None => false,
    }
}

/// Classify `line` against the `:`-separated `path` dir list.
pub(crate) fn resolve(line: &str, path: &str) -> Verdict {
    let Some(word) = first_word(line) else {
        return Verdict::No;
    };
    if BUILTINS.contains(&word.as_str()) {
        return Verdict::Builtin(word);
    }
    if is_path_like(&word) {
        let p = expand_home(&word);
        return if is_executable(&p) {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or(word);
            Verdict::Executable(name)
        } else {
            Verdict::No
        };
    }
    // `split_paths`, never `split(':')`: the separator is `;` on Windows, and
    // splitting a Windows PATH on `:` tears every entry in half at its drive
    // letter (`C:\bin` -> `C`, `\bin`), so nothing on PATH ever resolved.
    for dir in std::env::split_paths(path).filter(|d| !d.as_os_str().is_empty()) {
        if candidates(&word)
            .iter()
            .any(|c| is_executable(&dir.join(c)))
        {
            return Verdict::Executable(word);
        }
    }
    Verdict::No
}

/// Does `word` name a path, rather than a bare command to look up on PATH?
/// A parent component is the platform-neutral test: it catches `/usr/bin/rg`
/// and `./rg` everywhere, and `C:\tools\rg.exe` on Windows, where a check
/// for `/` alone saw a bare command word and searched PATH for it.
fn is_path_like(word: &str) -> bool {
    Path::new(word)
        .parent()
        .is_some_and(|p| !p.as_os_str().is_empty())
}

/// The file names a bare `word` could resolve to inside one PATH directory:
/// just `word` on Unix, plus each `PATHEXT` suffix on Windows, where `git`
/// on disk is `git.exe` and the extension is what makes it runnable at all.
fn candidates(word: &str) -> Vec<String> {
    #[cfg(unix)]
    {
        vec![word.to_string()]
    }
    #[cfg(not(unix))]
    {
        let mut v = vec![word.to_string()];
        v.extend(pathext().into_iter().map(|e| format!("{word}{e}")));
        v
    }
}

/// The extensions Windows considers directly runnable, lowercased. The
/// default matches what `cmd.exe` ships with when `PATHEXT` is unset.
#[cfg(not(unix))]
fn pathext() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_ascii_lowercase())
        .collect()
}

fn expand_home(word: &str) -> PathBuf {
    if let Some(rest) = word.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(word)
}

/// Executable regular file: the mode bit on Unix, and on Windows — which has
/// no such bit — an extension listed in `PATHEXT`. Existence alone is NOT
/// the signal there: it makes every `README.txt` on PATH look runnable.
fn is_executable(p: &Path) -> bool {
    let Ok(md) = std::fs::metadata(p) else {
        return false;
    };
    if !md.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        md.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let Some(ext) = p.extension() else {
            return false;
        };
        let ext = format!(".{}", ext.to_string_lossy().to_ascii_lowercase());
        pathext().contains(&ext)
    }
}

/// The PATH detection resolves against: hydrated login-shell PATH once
/// [`crate::shellprobe::init_probe`]'s probe lands, the process PATH until
/// then. The probe (one `$SHELL -ilc env`, shared with provider-key
/// discovery) is kicked off once from `main.rs` — this crate no longer runs
/// its own separate shell just for PATH.
pub(crate) fn effective_path() -> String {
    crate::shellprobe::effective_path()
}

#[cfg(test)]
#[path = "cmdcheck_tests.rs"]
mod tests;
