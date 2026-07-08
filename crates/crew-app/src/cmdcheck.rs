//! Is this line a runnable command? Powers the input bar's smart routing:
//! the first word must resolve to a real executable (hydrated login-shell
//! PATH, explicit path) or a shell builtin before crew will spawn a pane
//! for it — so typos hint instead of littering dead panes.
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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
    if word.contains('/') {
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
    for dir in path.split(':').filter(|d| !d.is_empty()) {
        if is_executable(&Path::new(dir).join(&word)) {
            return Verdict::Executable(word);
        }
    }
    Verdict::No
}

fn expand_home(word: &str) -> PathBuf {
    if let Some(rest) = word.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(word)
}

/// Executable regular file. On non-Unix there is no mode bit; existence of a
/// file is the best cheap signal.
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
        true
    }
}

/// Login-shell PATH captured once by [`init_shell_path`]. Dock-launched crew
/// inherits launchd's minimal PATH, so a command like `claude` in
/// `~/.local/bin` would *run* fine (spawns go through `$SHELL -c`) yet fail
/// detection without this.
static SHELL_PATH: OnceLock<String> = OnceLock::new();

/// Capture `$SHELL -lc 'printf %s "$PATH"'` on a background thread (the winit
/// thread must never block on a subprocess). `CREW_SHELL_ENV=0` skips it,
/// mirroring the broker's env hydration switch.
pub(crate) fn init_shell_path() {
    if std::env::var("CREW_SHELL_ENV").is_ok_and(|v| v == "0") {
        return;
    }
    std::thread::spawn(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let Ok(out) = std::process::Command::new(&shell)
            .args(["-lc", "printf %s \"$PATH\""])
            .output()
        else {
            return;
        };
        if !out.status.success() {
            return;
        }
        if let Ok(p) = String::from_utf8(out.stdout) {
            if !p.trim().is_empty() {
                let _ = SHELL_PATH.set(p);
            }
        }
    });
}

/// The PATH detection resolves against: hydrated login-shell PATH once it
/// lands, the process PATH until then.
pub(crate) fn effective_path() -> String {
    SHELL_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temp dir holding one executable `hit` and one plain file `miss`.
    fn fixture() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let hit = d.path().join("hit");
        std::fs::write(&hit, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hit, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        std::fs::write(d.path().join("miss"), "").unwrap();
        d
    }

    #[test]
    fn first_word_strips_env_prefixes_and_quotes() {
        assert_eq!(first_word("FOO=1 BAR=2 cargo test"), Some("cargo".into()));
        assert_eq!(first_word("\"hit\" --flag"), Some("hit".into()));
        assert_eq!(first_word("  ls -la"), Some("ls".into()));
        assert_eq!(
            first_word("FOO=1"),
            None,
            "only assignments → no command word"
        );
        assert_eq!(first_word(""), None);
    }

    #[test]
    fn resolve_finds_executables_on_the_given_path() {
        let d = fixture();
        let path = d.path().to_str().unwrap().to_string();
        assert_eq!(
            resolve("hit --flag", &path),
            Verdict::Executable("hit".into())
        );
        assert_eq!(resolve("miss", &path), Verdict::No, "non-executable file");
        assert_eq!(resolve("nosuch", &path), Verdict::No);
    }

    #[test]
    fn resolve_accepts_explicit_paths_and_rejects_bad_ones() {
        let d = fixture();
        let hit = d.path().join("hit");
        assert_eq!(
            resolve(hit.to_str().unwrap(), ""),
            Verdict::Executable("hit".into()),
            "absolute path bypasses PATH"
        );
        assert_eq!(resolve("./nosuch/prog", ""), Verdict::No);
    }

    #[test]
    fn resolve_flags_shell_builtins() {
        assert_eq!(
            resolve("export FOO=1", ""),
            Verdict::Builtin("export".into())
        );
        assert_eq!(
            resolve("source ~/.zshrc", ""),
            Verdict::Builtin("source".into())
        );
    }

    #[test]
    fn effective_path_falls_back_to_process_path() {
        // Hydration hasn't run in tests; must equal the process PATH, not panic.
        assert_eq!(effective_path(), std::env::var("PATH").unwrap_or_default());
    }
}
