//! A pane speaks UTF-8 even when nothing told crew which language to use.
//!
//! macOS hands an app started from the Dock no `LANG`, and a shell's rc file
//! often doesn't set one either (this user's has it commented out). Measured
//! 2026-11-02 in a tty with none: `ls` printed `caf??.txt`, zsh counted `é`
//! as two characters and its line editor deleted half of one, `locale
//! charmap` said US-ASCII, and less and git's pager showed `<C3><A9>`.
//! A pane started with `zsh -c` reads no rc file at all, so every agent CLI
//! ran that way. Terminal.app, iTerm2, alacritty, WezTerm and Ghostty all set
//! `LANG` when it is missing; so does crew now, from the language macOS is
//! set to. A locale the user chose (`LANG`, `LC_ALL` or `LC_CTYPE`) is left
//! alone.
use std::sync::OnceLock;

use portable_pty::CommandBuilder;

/// Used when the system's own locale has no UTF-8 form installed.
#[cfg(target_os = "macos")]
const FALLBACK: &str = "en_US.UTF-8";
#[cfg(not(target_os = "macos"))]
const FALLBACK: &str = "C.UTF-8";

/// Give `cmd` a UTF-8 `LANG` when it has no locale of its own.
pub(crate) fn settle(cmd: &mut CommandBuilder) {
    let set = |k: &str| cmd.get_env(k).is_some_and(|v| !v.is_empty());
    if cfg!(windows) || ["LANG", "LC_ALL", "LC_CTYPE"].into_iter().any(set) {
        return;
    }
    cmd.env("LANG", system());
}

/// The system's language as a UTF-8 locale, looked up once.
fn system() -> &'static str {
    static ONE: OnceLock<String> = OnceLock::new();
    ONE.get_or_init(|| {
        let exists = |l: &str| std::path::Path::new("/usr/share/locale").join(l).exists();
        pick(apple_locale().as_deref(), exists)
    })
}

/// `<language>_<region>.UTF-8` from macOS's `AppleLocale` (`en_GB`,
/// `de_DE@calendar=…`) when that locale is installed, else [`FALLBACK`].
fn pick(apple: Option<&str>, exists: impl Fn(&str) -> bool) -> String {
    let base = apple.map(|a| a.split('@').next().unwrap_or(a).trim());
    base.filter(|b| !b.is_empty())
        .map(|b| format!("{b}.UTF-8"))
        .filter(|l| exists(l))
        .unwrap_or_else(|| FALLBACK.to_string())
}

#[cfg(target_os = "macos")]
fn apple_locale() -> Option<String> {
    let out = std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(not(target_os = "macos"))]
fn apple_locale() -> Option<String> {
    None
}

#[cfg(test)]
#[path = "ptylocale_tests.rs"]
mod tests;
