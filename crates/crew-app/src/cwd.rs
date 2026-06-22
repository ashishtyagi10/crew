//! Working-directory tracking for the input bar: the bar's legend shows the
//! current directory, and a `cd` typed into the bar moves it — that's where new
//! shells (Cmd+T / `/shell`) then open.
use std::path::{Path, PathBuf};

use crate::app::CrewApp;

/// The directory Crew starts in: the process CWD, falling back to `$HOME`, `/`.
pub(crate) fn initial() -> PathBuf {
    std::env::current_dir()
        .ok()
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// `~`-abbreviated display string for `path`, e.g. `~/code/farx`.
pub(crate) fn display(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if !home.is_empty() {
            if s == home {
                return "~".to_string();
            }
            if let Some(rest) = s.strip_prefix(&format!("{home}/")) {
                return format!("~/{rest}");
            }
        }
    }
    s.into_owned()
}

/// If `line` is a `cd` command, return its argument (`""` means "go home").
/// `cd` alone or `cd <path>` match; anything else returns `None`.
pub(crate) fn cd_arg(line: &str) -> Option<&str> {
    let t = line.trim();
    if t == "cd" {
        Some("")
    } else {
        t.strip_prefix("cd ").map(str::trim)
    }
}

/// Resolve `cd arg` against `base`: empty/`~` → `$HOME`; `~/x` expanded; an
/// absolute path kept; a relative path joined onto `base`. Returns the canonical
/// path only when it resolves to an existing directory.
pub(crate) fn resolve(base: &Path, arg: &str) -> Option<PathBuf> {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let target = if arg.is_empty() || arg == "~" {
        home()?
    } else if let Some(rest) = arg.strip_prefix("~/") {
        home()?.join(rest)
    } else {
        let p = Path::new(arg);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            base.join(p)
        }
    };
    let canon = target.canonicalize().ok()?;
    canon.is_dir().then_some(canon)
}

impl CrewApp {
    /// Point Crew at `dir`: update the tracked cwd and the input-bar legend.
    pub(crate) fn set_cwd(&mut self, dir: PathBuf) {
        self.input.cwd = display(&dir);
        self.cwd = dir;
    }

    /// If `line` is a `cd` command, change directory (when the target exists)
    /// and return `true` so it is not forwarded to a terminal pane.
    pub(crate) fn try_change_dir(&mut self, line: &str) -> bool {
        let Some(arg) = cd_arg(line) else {
            return false;
        };
        if let Some(dir) = resolve(&self.cwd, arg) {
            self.set_cwd(dir);
            self.redraw();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cd_arg_parses() {
        assert_eq!(cd_arg("cd"), Some(""));
        assert_eq!(cd_arg("cd /tmp"), Some("/tmp"));
        assert_eq!(cd_arg("  cd   foo/bar "), Some("foo/bar"));
        assert_eq!(cd_arg("cdx"), None);
        assert_eq!(cd_arg("ls"), None);
    }

    #[test]
    fn resolve_relative_and_absolute() {
        let base = std::env::temp_dir().canonicalize().unwrap();
        // "." resolves back to base.
        assert_eq!(resolve(&base, "."), Some(base.clone()));
        // an absolute existing dir is kept.
        assert_eq!(resolve(&base, base.to_str().unwrap()), Some(base.clone()));
        // a non-existent path resolves to None.
        assert_eq!(resolve(&base, "definitely-not-here-xyz"), None);
    }

    #[test]
    fn display_abbreviates_home() {
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            assert_eq!(display(&home), "~");
            assert_eq!(display(&home.join("code")), "~/code");
        }
        assert_eq!(display(Path::new("/etc")), "/etc");
    }
}
