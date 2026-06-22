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

/// The directory to launch in: the `saved` config path when it still exists as a
/// directory, otherwise [`initial`]. Lets Crew reopen where it was last left.
pub(crate) fn resolved_start(saved: Option<&str>) -> PathBuf {
    saved
        .map(PathBuf::from)
        .and_then(|p| p.canonicalize().ok())
        .filter(|p| p.is_dir())
        .unwrap_or_else(initial)
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

/// Resolve `cd arg` against `base`: `$VAR`/`${VAR}` are expanded first, then
/// empty/`~` → `$HOME`; `~/x` expanded; an absolute path kept; a relative path
/// joined onto `base`. Returns the canonical path only when it's a directory.
pub(crate) fn resolve(base: &Path, arg: &str) -> Option<PathBuf> {
    let expanded = crate::envexpand::expand_env(arg);
    let arg = expanded.as_str();
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
    /// Point Crew at `dir`: remember the current dir (for `cd -`), update the
    /// tracked cwd and input-bar legend, and persist it so the next launch
    /// reopens here.
    pub(crate) fn set_cwd(&mut self, dir: PathBuf) {
        if dir != self.cwd && !self.cwd.as_os_str().is_empty() {
            self.prev_cwd = self.cwd.clone();
        }
        self.config.last_dir = Some(dir.to_string_lossy().into_owned());
        self.config.save();
        self.input.cwd = dir.clone();
        self.cwd = dir;
    }

    /// If `line` is a `cd` command, change directory (when the target exists)
    /// and return `true` so it is not forwarded to a terminal pane. `cd -`
    /// toggles back to the previous directory.
    pub(crate) fn try_change_dir(&mut self, line: &str) -> bool {
        let Some(arg) = cd_arg(line) else {
            return false;
        };
        let target = if arg == "-" {
            (!self.prev_cwd.as_os_str().is_empty()).then(|| self.prev_cwd.clone())
        } else {
            resolve(&self.cwd, arg)
        };
        match target {
            Some(dir) => {
                self.set_cwd(dir);
                self.redraw();
            }
            None if arg == "-" => self.set_status("cd: no previous directory"),
            None => self.set_status(format!("cd: no such directory: {arg}")),
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
    fn resolve_expands_env_var() {
        let base = std::env::temp_dir().canonicalize().unwrap();
        std::env::set_var("CREW_RESOLVE_DIR", base.to_str().unwrap());
        // `$VAR` expands to an absolute existing dir.
        assert_eq!(resolve(Path::new("/"), "$CREW_RESOLVE_DIR"), Some(base));
    }

    #[test]
    fn resolved_start_prefers_valid_saved_dir() {
        let base = std::env::temp_dir().canonicalize().unwrap();
        // a saved dir that exists is used
        assert_eq!(resolved_start(Some(base.to_str().unwrap())), base);
        // a missing saved dir, or none, falls back to the process cwd
        let fallback = initial();
        assert_eq!(resolved_start(Some("/no/such/dir/xyz")), fallback);
        assert_eq!(resolved_start(None), fallback);
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
