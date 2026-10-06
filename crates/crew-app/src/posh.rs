//! oh-my-posh in a Windows PowerShell pane (2026-10-06, the user: "crew should
//! find/install and use oh-my-posh on windows, I do use oh-my-zsh on mac
//! though").
//!
//! On a Mac a pane is the user's login zsh, so oh-my-zsh arrives with their
//! `~/.zshrc`. A Windows pane was bare `powershell.exe` and its `PS C:\>`. So
//! a PowerShell pane now starts oh-my-posh, the prompt engine Windows users
//! reach for, in crew's own theme ([`THEME`]): oh-my-zsh's default look
//! (robbyrussell), in the ANSI palette so it wears whatever theme crew wears.
//!
//! * **Find** — `PATH`, then where each installer leaves it ([`find`]).
//! * **Install** — once, in the open: when it is nowhere, the first
//!   PowerShell pane runs winget before its first prompt, where the user can
//!   watch. A marker remembers the attempt, so crew never installs it again —
//!   uninstalling it is a choice crew respects.
//! * **Use** — only over PowerShell's DEFAULT prompt: a profile that set its
//!   own (its own oh-my-posh theme, Starship, a hand-written `prompt`) wins.
//!   And through `--eval`, which evaluates the init in memory: plain
//!   `init pwsh` writes a `.ps1` and runs it, which Windows' default
//!   execution policy refuses.
use std::path::{Path, PathBuf};

/// The executable's file name.
const EXE: &str = "oh-my-posh.exe";

/// crew's prompt: oh-my-zsh's robbyrussell (`➜  dir git:(branch) ✗`) in ANSI
/// colour names rather than hex, so the prompt follows crew's theme the way
/// a zsh prompt does. No Nerd Font icon: crew's embedded face has none.
pub(crate) const THEME: &str = include_str!("../../../assets/oh-my-posh/crew.omp.json");

/// Whether `shell` is PowerShell — Windows PowerShell or PowerShell 7 — by
/// its file name, under either separator.
pub(crate) fn is_powershell(shell: &str) -> bool {
    let name = shell.rsplit(['/', '\\']).next().unwrap_or(shell);
    let name = name.to_ascii_lowercase();
    matches!(
        name.strip_suffix(".exe").unwrap_or(&name),
        "powershell" | "pwsh"
    )
}

/// Where oh-my-posh is: every `PATH` entry, then the places its installers
/// leave it whether or not `PATH` was updated — winget's MSIX app alias,
/// the old per-user installer, Scoop's shims and Chocolatey's bin. `env`
/// reads an environment variable, `exists` a file.
pub(crate) fn find(
    env: impl Fn(&str) -> Option<String>,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let known: [(&str, &[&str]); 4] = [
        ("LOCALAPPDATA", &["Microsoft", "WindowsApps"]),
        ("LOCALAPPDATA", &["Programs", "oh-my-posh", "bin"]),
        ("USERPROFILE", &["scoop", "shims"]),
        ("ProgramData", &["chocolatey", "bin"]),
    ];
    crate::winexe::find(EXE, &known, env, exists)
}

/// A PowerShell single-quoted literal: nothing inside is expanded, and a
/// quote is written twice.
fn quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The script a PowerShell pane runs after its profile: settle on an
/// oh-my-posh (`exe`, else whatever `PATH` holds now, else — with `install`
/// — winget's), then init it with `config` over the default prompt only.
/// It ends on a command that cannot fail: the prompt reads `$?`, and naming
/// a variable that was never set (`__crewAlias` with nothing to install)
/// left it false, so every pane opened on a red arrow. A wildcard that
/// matches nothing is not an error.
pub(crate) fn script(exe: Option<&Path>, install: bool, config: &Path) -> String {
    let lookup = "(Get-Command oh-my-posh -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source";
    let mut s = format!(
        "$__crewPosh = {}\nif (-not $__crewPosh) {{ $__crewPosh = {lookup} }}\n",
        exe.map_or("$null".to_string(), |p| quoted(&p.to_string_lossy()))
    );
    if install {
        s.push_str(&format!(
            "if (-not $__crewPosh) {{\n\
             \x20 if (Get-Command winget -ErrorAction SilentlyContinue) {{\n\
             \x20   Write-Host 'crew: installing oh-my-posh for this prompt (winget, once)...'\n\
             \x20   winget install --id JanDeDobbeleer.OhMyPosh --exact --source winget --accept-package-agreements --accept-source-agreements --disable-interactivity\n\
             \x20   $__crewPosh = {lookup}\n\
             \x20   $__crewAlias = Join-Path $env:LOCALAPPDATA 'Microsoft\\WindowsApps\\{EXE}'\n\
             \x20   if (-not $__crewPosh -and (Test-Path -LiteralPath $__crewAlias)) {{ $__crewPosh = $__crewAlias }}\n\
             \x20 }} else {{\n\
             \x20   Write-Host 'crew: winget is missing, so oh-my-posh was not installed: https://ohmyposh.dev/docs/installation/windows'\n\
             \x20 }}\n\
             }}\n"
        ));
    }
    s.push_str(&format!(
        "if ($__crewPosh) {{\n\
         \x20 $__crewPrompt = (Get-Command prompt -CommandType Function -ErrorAction SilentlyContinue).ScriptBlock\n\
         \x20 if (-not $__crewPrompt -or \"$__crewPrompt\" -like '*nestedPromptLevel + 1*') {{\n\
         \x20   & $__crewPosh init pwsh --config {} --eval | Invoke-Expression\n\
         \x20 }}\n\
         }}\n\
         Remove-Variable -Name __crew*\n",
        quoted(&config.to_string_lossy())
    ));
    s
}

/// `script` as PowerShell's `-EncodedCommand` wants it: base64 of UTF-16LE.
/// Encoded, no quote or `$` in it can be mangled on the way through the
/// Windows command line.
pub(crate) fn encode(script: &str) -> String {
    use base64::Engine as _;
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// PowerShell's arguments for a pane that runs [`script`] after the
/// profile and stays open.
pub(crate) fn pane_args(script: &str) -> Vec<String> {
    ["-NoLogo", "-NoExit", "-EncodedCommand"]
        .into_iter()
        .map(String::from)
        .chain([encode(script)])
        .collect()
}

/// The arguments a PowerShell pane starts with, keeping crew's theme in
/// `dir`: the theme written where oh-my-posh can read it, and the install
/// armed only the first time oh-my-posh is found nowhere (`env` and
/// `exists` as in [`find`]). Empty — a plain PowerShell — when the theme
/// cannot be written.
pub(crate) fn args_in(
    dir: &Path,
    env: impl Fn(&str) -> Option<String>,
    exists: impl Fn(&Path) -> bool,
) -> Vec<String> {
    let config = dir.join("crew.omp.json");
    if std::fs::read_to_string(&config).ok().as_deref() != Some(THEME)
        && std::fs::create_dir_all(dir)
            .and_then(|()| std::fs::write(&config, THEME))
            .is_err()
    {
        return Vec::new();
    }
    let exe = find(env, exists);
    let marker = dir.join("oh-my-posh-install-tried");
    let install = exe.is_none() && !marker.exists() && std::fs::write(&marker, "").is_ok();
    pane_args(&script(exe.as_deref(), install, &config))
}

/// [`args_in`] for this machine: crew's config directory, the real
/// environment and file system.
#[cfg(windows)]
pub(crate) fn args() -> Vec<String> {
    match dirs::config_dir() {
        Some(d) => args_in(
            &d.join("crew"),
            |k| std::env::var(k).ok(),
            crate::winexe::present,
        ),
        None => Vec::new(),
    }
}

#[cfg(test)]
#[path = "posh_tests.rs"]
mod tests;
