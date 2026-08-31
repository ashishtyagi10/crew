//! Working-directory tracking for the input bar: the bar's legend shows the
//! current directory, and a `cd` typed into the bar moves it — that's where new
//! shells (Cmd+T / `/shell`) then open.
pub(crate) use crate::cwdshow::*;
use std::path::{Path, PathBuf};

use crate::app::CrewApp;

/// Whether the process CWD is one no user chose, so Crew should start at home
/// instead of inheriting it.
///
/// A launcher, not a shell, picks these:
///
/// * **the directory holding our own executable** — this is what Explorer and
///   a Start-menu shortcut set on Windows. It made every pane open inside the
///   unzipped release folder, so the PowerShell prompt read
///   `PS C:\Users\me\Downloads\crew-v0.17.10-x86_64-pc-windows-msvc>`: the
///   exe's own directory, reported as the place you are working.
/// * **the filesystem root** — what launchd hands a macOS Dock launch. The
///   same class of bug; see the broker's `spawn_in(pane cwd)` fix.
///
/// A real terminal launch lands somewhere the user chose and is left alone —
/// including, deliberately, the case where they `cd` into the release folder
/// themselves and run it from there.
fn is_launcher_cwd(cwd: &Path) -> bool {
    if cwd.parent().is_none() {
        return true; // `/`, or a bare drive root like `C:\`
    }
    std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::parent)
        .is_some_and(|exe_dir| exe_dir == cwd)
}

/// The directory Crew starts in: the process CWD when a person chose it, else
/// the user's home directory, else the root.
///
/// `dirs::home_dir` rather than `$HOME`: that variable is a POSIX convention
/// and is normally **unset on Windows**, where the home directory is
/// `%USERPROFILE%`. The old fallback could therefore never fire on the one
/// platform whose launcher most needed it.
pub(crate) fn initial() -> PathBuf {
    initial_from(std::env::current_dir().ok())
}

/// [`initial`] with the process CWD injected, so the decision is testable.
///
/// Taking the real CWD would make the test vacuous: under `cargo test` the
/// working directory is the crate root, never the exe's folder, so an
/// assertion that `initial()` avoids the exe folder would hold no matter what
/// the function did.
fn initial_from(cwd: Option<PathBuf>) -> PathBuf {
    cwd.filter(|c| !is_launcher_cwd(c.as_path()))
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

/// `canonicalize` + [`simplified`] — the exact pair the product applies, for
/// tests that need to state an expected path.
///
/// Windows CI caught six tests computing their expectation with a bare
/// `canonicalize()`, which there yields the verbatim `\\?\C:\…` form the
/// product now strips. The assertions were wrong, not the code — but a test
/// that hand-rolls half of what it is checking will drift again, so they all
/// go through this.
#[cfg(test)]
pub(crate) fn canonical(path: &Path) -> PathBuf {
    simplified(path.canonicalize().expect("canonicalize"))
}

/// The directory to launch in: the `saved` config path when it still exists as a
/// directory, otherwise [`initial`]. Lets Crew reopen where it was last left.
pub(crate) fn resolved_start(saved: Option<&str>) -> PathBuf {
    saved
        .map(PathBuf::from)
        .and_then(|p| p.canonicalize().ok())
        .map(simplified)
        .filter(|p| p.is_dir())
        .unwrap_or_else(initial)
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
/// empty/`~` → the home directory; `~/x` (or `~\x`) expanded; an absolute path
/// kept; a relative path joined onto `base`. Returns the canonical path only
/// when it's a directory.
///
/// Home comes from `dirs::home_dir`, not `$HOME` — the variable is a POSIX
/// convention Windows does not set, so `cd` and `cd ~` did nothing at all
/// there.
pub(crate) fn resolve(base: &Path, arg: &str) -> Option<PathBuf> {
    let expanded = crate::envexpand::expand_env(arg);
    let arg = expanded.as_str();
    let target = if arg.is_empty() || arg == "~" {
        dirs::home_dir()?
    } else if let Some(rest) = arg.strip_prefix("~/").or_else(|| arg.strip_prefix("~\\")) {
        dirs::home_dir()?.join(rest)
    } else {
        let p = Path::new(arg);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            base.join(p)
        }
    };
    let canon = simplified(target.canonicalize().ok()?);
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
#[path = "cwd_tests.rs"]
mod tests;
