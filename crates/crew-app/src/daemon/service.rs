//! Making the resident resident: an opt-in OS service that starts the daemon at login.
//!
//! This is `crew daemon install` and nothing else. NO release, update, or app launch may install
//! it — a background service the user did not ask for is the one failure mode that turns a bad
//! build into a login loop instead of a `/update`. The command is the whole consent.
//!
//! macOS gets a launchd user agent, Linux a systemd user unit. Both are per-user: no sudo, no
//! system-wide daemon, nothing outside `$HOME`.
use std::path::{Path, PathBuf};

/// The service's identity. The unit file is named from it, so install and uninstall cannot
/// disagree about what to look for.
///
/// Gated exactly like the two unit builders that read it: Windows has no service integration
/// yet, so on that target this constant has no reader and `-D warnings` in CI failed the build
/// on it — every release since 0.18.18.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
pub(crate) const LABEL: &str = "com.crew.daemon";

/// What a platform's service manager needs written and run.
#[derive(Debug, PartialEq)]
pub(crate) struct Unit {
    /// Where the file goes, relative to `$HOME`.
    pub rel_path: String,
    /// The file's contents.
    pub body: String,
    /// The command that activates it, for the user to see (we run it, but it is also what they
    /// would type, and what they need if they later want it gone).
    pub activate: Vec<String>,
    pub deactivate: Vec<String>,
}

/// The launchd agent. `RunAtLoad` starts it at login; `KeepAlive` brings it back if it dies.
///
/// `exe` MUST be absolute: a launchd agent starts with a minimal environment and does not
/// inherit the login shell's PATH, so a bare `crew` here would work in a terminal and fail
/// silently at boot — the classic way a Mac launch agent looks installed and never runs.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn macos_unit(exe: &Path) -> Unit {
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>daemon</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>
"#,
        exe = exe.display()
    );
    let path = format!("Library/LaunchAgents/{LABEL}.plist");
    Unit {
        rel_path: path.clone(),
        body,
        activate: vec!["launchctl".into(), "load".into(), "-w".into(), path.clone()],
        deactivate: vec!["launchctl".into(), "unload".into(), "-w".into(), path],
    }
}

/// The systemd user unit. `default.target` is the user-session target, so it starts at login
/// without needing lingering enabled.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn linux_unit(exe: &Path) -> Unit {
    let body = format!(
        "[Unit]\n\
         Description=crew resident daemon\n\n\
         [Service]\n\
         ExecStart={exe} daemon run\n\
         Restart=on-failure\n\n\
         [Install]\n\
         WantedBy=default.target\n",
        exe = exe.display()
    );
    Unit {
        rel_path: format!(".config/systemd/user/{LABEL}.service"),
        body,
        activate: vec![
            "systemctl".into(),
            "--user".into(),
            "enable".into(),
            "--now".into(),
            format!("{LABEL}.service"),
        ],
        deactivate: vec![
            "systemctl".into(),
            "--user".into(),
            "disable".into(),
            "--now".into(),
            format!("{LABEL}.service"),
        ],
    }
}

/// This platform's unit, or `None` where no service integration exists yet. Saying so beats
/// writing a file nothing will ever read.
pub(crate) fn unit_for_host(exe: &Path) -> Option<Unit> {
    #[cfg(target_os = "macos")]
    return Some(macos_unit(exe));
    #[cfg(target_os = "linux")]
    return Some(linux_unit(exe));
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = exe;
        None
    }
}

/// Write the unit under `home`. Returns its absolute path. Pure filesystem work — activation is
/// the caller's separate step, so a test can install without asking the OS to run anything.
pub(crate) fn write_unit(home: &Path, unit: &Unit) -> std::io::Result<PathBuf> {
    let path = home.join(&unit.rel_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, &unit.body)?;
    Ok(path)
}

/// Remove the unit under `home`. `Ok(false)` when there was nothing installed — a state worth
/// reporting plainly rather than as an error.
pub(crate) fn remove_unit(home: &Path, unit: &Unit) -> std::io::Result<bool> {
    let path = home.join(&unit.rel_path);
    if !path.exists() {
        return Ok(false);
    }
    std::fs::remove_file(&path)?;
    Ok(true)
}

/// Is the unit file present?
pub(crate) fn is_installed(home: &Path, unit: &Unit) -> bool {
    home.join(&unit.rel_path).exists()
}

/// Run an activation/deactivation command from `home`, so the relative unit path in it resolves.
/// Failure is reported, never fatal: the file is written either way, and a user can run the
/// command themselves.
pub(crate) fn run_step(home: &Path, argv: &[String]) -> Result<(), String> {
    let Some((program, args)) = argv.split_first() else {
        return Err("empty command".into());
    };
    let mut cmd = std::process::Command::new(program);
    crew_hive::childproc::no_console_window(&mut cmd);
    cmd.args(args).current_dir(home);
    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
