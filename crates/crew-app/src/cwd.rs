//! Working-directory tracking for the input bar: the bar's legend shows the
//! current directory, and a `cd` typed into the bar moves it — that's where new
//! shells (Cmd+T / `/shell`) then open.
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

/// Strip Windows' extended-length `\\?\` prefix from a canonicalized path.
///
/// `std::fs::canonicalize` returns *verbatim* paths on Windows —
/// `C:\Users\me\code` comes back as `\\?\C:\Users\me\code`. Crew canonicalizes
/// both the saved start directory and every `cd` target, so that form ended up
/// in the input-bar legend, in each pane's spawn directory, and in the
/// PowerShell prompt: four extra characters of noise on a path that was
/// already being shown in full, because the verbatim prefix also stops
/// [`abbreviate`] from ever recognising home.
///
/// Only the plain-drive form is unwrapped. `\\?\UNC\server\share` is left
/// exactly as it is: that prefix is load-bearing for network paths, and
/// rewriting it is how you break someone's mapped drive.
fn strip_verbatim(path: &str) -> &str {
    let Some(rest) = path.strip_prefix(r"\\?\") else {
        return path;
    };
    let mut chars = rest.chars();
    let is_drive = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.next() == Some(':')
        && chars.next() == Some('\\');
    if is_drive {
        rest
    } else {
        path
    }
}

/// [`strip_verbatim`] applied to an owned path.
pub(crate) fn simplified(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    match strip_verbatim(&s) {
        stripped if stripped.len() == s.len() => path.clone(),
        stripped => PathBuf::from(stripped),
    }
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

/// `~`-abbreviated display string for `path`, e.g. `~/code/crew`.
///
/// Windows needed two fixes here, and together they are why the legend read as
/// a "long name": the home directory came from `$HOME`, which Windows does not
/// set, and the separator was hardcoded to `/`. So `C:\Users\me\code` never
/// abbreviated and the bar showed the whole absolute path.
pub(crate) fn display(path: &Path) -> String {
    let s = path.to_string_lossy();
    match dirs::home_dir() {
        Some(home) => abbreviate(&s, &home.to_string_lossy(), std::path::MAIN_SEPARATOR),
        None => s.into_owned(),
    }
}

/// The abbreviation itself, with home and the separator passed in.
///
/// Parameterised so the **Windows** behaviour is testable from any machine.
/// A test that reads `MAIN_SEPARATOR` proves nothing about Windows when it
/// runs on macOS, where that constant is already `/` — the hardcoded `/` this
/// replaces would have passed such a test every time while leaving every
/// Windows path unabbreviated.
fn abbreviate(path: &str, home: &str, sep: char) -> String {
    if home.is_empty() {
        return path.to_string();
    }
    if path == home {
        return "~".to_string();
    }
    // Trim a trailing separator first: a home of `C:\` would otherwise be
    // matched as the prefix `C:\\`, which no path carries.
    let base = home.trim_end_matches(sep);
    match path.strip_prefix(&format!("{base}{sep}")) {
        Some(rest) => format!("~{sep}{rest}"),
        None => path.to_string(),
    }
}

/// Truncate `s` from the left to at most `max` columns, keeping the tail (the
/// most specific path component) behind a leading `…`. Used so a deep cwd legend
/// shows the current directory rather than being clipped at the root.
pub(crate) fn fit_legend(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max || max == 0 {
        return s.to_string();
    }
    let tail: String = s.chars().skip(n - max.saturating_sub(1)).collect();
    format!("…{tail}")
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
        let base = canonical(&std::env::temp_dir());
        // "." resolves back to base.
        assert_eq!(resolve(&base, "."), Some(base.clone()));
        // an absolute existing dir is kept.
        assert_eq!(resolve(&base, base.to_str().unwrap()), Some(base.clone()));
        // a non-existent path resolves to None.
        assert_eq!(resolve(&base, "definitely-not-here-xyz"), None);
    }

    #[test]
    fn resolve_expands_env_var() {
        let base = canonical(&std::env::temp_dir());
        std::env::set_var("CREW_RESOLVE_DIR", base.to_str().unwrap());
        // `$VAR` expands to an absolute existing dir.
        assert_eq!(resolve(Path::new("/"), "$CREW_RESOLVE_DIR"), Some(base));
    }

    #[test]
    fn resolved_start_prefers_valid_saved_dir() {
        let base = canonical(&std::env::temp_dir());
        // a saved dir that exists is used
        assert_eq!(resolved_start(Some(base.to_str().unwrap())), base);
        // a missing saved dir, or none, falls back to the process cwd
        let fallback = initial();
        assert_eq!(resolved_start(Some("/no/such/dir/xyz")), fallback);
        assert_eq!(resolved_start(None), fallback);
    }

    #[test]
    fn fit_legend_keeps_tail_behind_ellipsis() {
        // fits → unchanged.
        assert_eq!(fit_legend("~/code/crew", 20), "~/code/crew");
        // too long → leading ellipsis + the last (max-1) chars (the deep tail).
        assert_eq!(fit_legend("~/a/b/c/deep", 6), "…/deep");
        // a zero budget is a no-op rather than a panic.
        assert_eq!(fit_legend("~/x", 0), "~/x");
    }

    /// `$HOME` here would make this a no-op on Windows — which is exactly how
    /// the bug shipped: the legend showed `C:\Users\me\code` in full because
    /// neither the home lookup nor the `/` separator applied there.
    #[test]
    fn display_abbreviates_home_on_this_platform() {
        let home = dirs::home_dir().expect("every supported platform has a home dir");
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(display(&home), "~");
        assert_eq!(display(&home.join("code")), format!("~{sep}code"));
        assert_eq!(
            display(&home.join("code").join("crew")),
            format!("~{sep}code{sep}crew")
        );
        // A path outside home is untouched.
        let outside = home
            .parent()
            .unwrap_or(&home)
            .join("definitely-not-home-xyz");
        assert_eq!(display(&outside), outside.to_string_lossy());
    }

    /// `std::fs::canonicalize` hands back `\\?\C:\Users\me` on Windows, and crew
    /// canonicalizes both the saved start directory and every `cd` target — so
    /// that prefix reached the legend, the pane spawn directory and the shell
    /// prompt, and blocked the `~` abbreviation on top.
    #[test]
    fn the_windows_verbatim_prefix_is_unwrapped() {
        assert_eq!(strip_verbatim(r"\\?\C:\Users\me"), r"C:\Users\me");
        assert_eq!(strip_verbatim(r"\\?\D:\"), r"D:\");
        // Already plain, or POSIX — untouched.
        assert_eq!(strip_verbatim(r"C:\Users\me"), r"C:\Users\me");
        assert_eq!(strip_verbatim("/home/me"), "/home/me");
        // UNC keeps its prefix: it is load-bearing for network shares.
        assert_eq!(
            strip_verbatim(r"\\?\UNC\server\share"),
            r"\\?\UNC\server\share"
        );
        // Anything else after the prefix is left alone rather than guessed at.
        assert_eq!(strip_verbatim(r"\\?\Volume{abc}\x"), r"\\?\Volume{abc}\x");
    }

    /// The two fixes have to compose: unwrapping the prefix is what lets the
    /// abbreviation recognise home in the first place.
    #[test]
    fn an_unwrapped_windows_path_then_abbreviates() {
        let raw = r"\\?\C:\Users\me\code\crew";
        assert_eq!(
            abbreviate(strip_verbatim(raw), r"C:\Users\me", '\\'),
            r"~\code\crew"
        );
    }

    /// The Windows half of the legend bug, proved from any machine: home came
    /// from `$HOME` (unset on Windows) and the separator was a hardcoded `/`,
    /// so `C:\Users\me\code` never abbreviated and the bar showed the whole
    /// absolute path — the "long name" in the report.
    #[test]
    fn windows_paths_abbreviate_with_a_backslash() {
        assert_eq!(abbreviate(r"C:\Users\me", r"C:\Users\me", '\\'), "~");
        assert_eq!(
            abbreviate(r"C:\Users\me\code", r"C:\Users\me", '\\'),
            r"~\code"
        );
        assert_eq!(
            abbreviate(r"C:\Users\me\code\crew", r"C:\Users\me", '\\'),
            r"~\code\crew"
        );
        // A different user's directory is not "home"; prefix matching must not
        // fire on a partial component.
        assert_eq!(
            abbreviate(r"C:\Users\meredith\x", r"C:\Users\me", '\\'),
            r"C:\Users\meredith\x"
        );
        // A drive-root home keeps exactly one separator.
        assert_eq!(abbreviate(r"C:\code", r"C:\", '\\'), r"~\code");
    }

    #[test]
    fn posix_paths_abbreviate_with_a_slash() {
        assert_eq!(abbreviate("/home/me", "/home/me", '/'), "~");
        assert_eq!(abbreviate("/home/me/code", "/home/me", '/'), "~/code");
        assert_eq!(
            abbreviate("/home/mere/x", "/home/me", '/'),
            "/home/mere/x",
            "a partial component match must not abbreviate"
        );
        assert_eq!(abbreviate("/etc", "/home/me", '/'), "/etc");
        // No home known → the path is returned untouched rather than mangled.
        assert_eq!(abbreviate("/etc", "", '/'), "/etc");
    }

    /// The reported bug: launched from Explorer, Windows sets the CWD to the
    /// folder holding `crew.exe`, so every pane opened inside the unzipped
    /// release directory and the prompt read
    /// `PS C:\Users\me\Downloads\crew-v0.17.10-x86_64-pc-windows-msvc>`.
    #[test]
    fn the_directory_holding_our_own_exe_is_not_a_place_to_work() {
        let exe_dir = std::env::current_exe()
            .expect("current_exe")
            .parent()
            .expect("exe has a parent")
            .to_path_buf();
        assert!(
            is_launcher_cwd(&exe_dir),
            "{exe_dir:?} holds the running binary — a launcher put us here, \
             not a person"
        );
        assert_eq!(
            initial_from(Some(exe_dir.clone())),
            dirs::home_dir().expect("home dir"),
            "started in the exe's own folder — panes would open inside the \
             unzipped release download instead of somewhere useful"
        );
        // …while a directory a person picked is passed straight through.
        let chosen = std::env::temp_dir();
        assert_eq!(initial_from(Some(chosen.clone())), chosen);
        // …and no CWD at all still lands somewhere real.
        assert_eq!(initial_from(None), dirs::home_dir().expect("home dir"));
    }

    /// …and the root, which is what launchd hands a macOS Dock launch.
    #[test]
    fn a_filesystem_root_is_not_a_place_to_work() {
        assert!(is_launcher_cwd(Path::new(std::path::MAIN_SEPARATOR_STR)));
        if cfg!(windows) {
            assert!(is_launcher_cwd(Path::new("C:\\")));
        }
    }

    /// But a directory the user actually chose is kept — including the release
    /// folder itself, if they `cd`'d there and ran it by hand.
    #[test]
    fn a_directory_a_person_chose_is_kept() {
        let tmp = std::env::temp_dir();
        assert!(
            !is_launcher_cwd(&tmp),
            "a normal directory must be left alone, or every launch would \
             ignore where it was started from"
        );
    }

    #[test]
    fn cd_home_resolves_without_a_posix_home_variable() {
        let home = dirs::home_dir().expect("home dir");
        let canon = Some(canonical(&home));
        // `cd`, `cd ~` both mean home — via dirs, not $HOME.
        assert_eq!(resolve(Path::new(std::path::MAIN_SEPARATOR_STR), ""), canon);
        assert_eq!(
            resolve(Path::new(std::path::MAIN_SEPARATOR_STR), "~"),
            canon
        );
    }
}
