//! Finding a Windows program where its user would expect it to be found:
//! every `PATH` entry, then the folders its installers leave it in whether
//! or not `PATH` was updated (a GUI launched before an install still holds
//! the old `PATH`). Shared by oh-my-posh (`posh`) and PowerShell 7 below.
use std::path::{Path, PathBuf};

/// `name` on `PATH`, else in the first of `known` — `(variable, folders
/// under it)` — that holds it. `env` reads an environment variable,
/// `exists` a file ([`present`] on the real machine).
pub(crate) fn find(
    name: &str,
    known: &[(&str, &[&str])],
    env: impl Fn(&str) -> Option<String>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let on_path = env("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    let under = known.iter().filter_map(|(var, rel)| {
        env(var).map(|root| rel.iter().fold(PathBuf::from(root), |p, r| p.join(r)))
    });
    on_path
        .into_iter()
        .chain(under)
        .map(|dir| dir.join(name))
        .find(|exe| exists(exe))
}

/// Whether a program file is at `p`. By the entry itself, not what it
/// points to: winget's and the Store's programs are app-execution aliases
/// in `WindowsApps`, reparse points a following lookup cannot open.
pub(crate) fn present(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok_and(|m| !m.is_dir())
}

/// PowerShell 7 (`pwsh.exe`): `PATH`, then its MSI's folder, the winget or
/// Store alias, Scoop's shims and the .NET global tool.
pub(crate) fn pwsh(
    env: impl Fn(&str) -> Option<String>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let known: [(&str, &[&str]); 4] = [
        ("ProgramFiles", &["PowerShell", "7"]),
        ("LOCALAPPDATA", &["Microsoft", "WindowsApps"]),
        ("USERPROFILE", &["scoop", "shims"]),
        ("USERPROFILE", &[".dotnet", "tools"]),
    ];
    find("pwsh.exe", &known, env, exists)
}

/// The shell a new Windows pane opens (2026-10-06, the user: "use pwsh 7
/// instead of powershell.exe if installed"): `$SHELL` when the user named
/// one, else PowerShell 7 when it is installed, else Windows PowerShell,
/// which every Windows has.
pub(crate) fn pane_shell(shell_var: Option<String>, pwsh: Option<PathBuf>) -> String {
    shell_var
        .filter(|s| !s.is_empty())
        .or_else(|| pwsh.map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "powershell.exe".to_string())
}

#[cfg(test)]
#[path = "winexe_tests.rs"]
mod tests;
