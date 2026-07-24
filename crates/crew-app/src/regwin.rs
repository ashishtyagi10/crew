//! Windows Start-menu registration: `.lnk` shortcut + staleness marker.
//! Split from `appregister.rs` (child module).
// `WinTarget` (the only consumer of these imports) is gated to Windows;
// `win_marker_content` names its paths fully qualified.
#[cfg(target_os = "windows")]
use super::*;

/// Contents of the sidecar staleness marker for the Start-menu shortcut
/// (.lnk files aren't cheaply parseable, so we track what we wrote).
// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn win_marker_content(exe: &std::path::Path, version: &str) -> String {
    format!("{version}|{}", exe.display())
}

#[cfg(target_os = "windows")]
pub struct WinTarget {
    /// `%APPDATA%\Microsoft\Windows\Start Menu\Programs`.
    pub programs_dir: PathBuf,
    /// Sidecar marker recording version|exe, kept in crew's config dir.
    pub marker: PathBuf,
    pub exe: PathBuf,
    pub version: String,
}

#[cfg(target_os = "windows")]
pub fn is_stale_windows(t: &WinTarget) -> bool {
    let marker_ok = std::fs::read_to_string(&t.marker)
        .map(|m| m == win_marker_content(&t.exe, &t.version))
        .unwrap_or(false);
    !(marker_ok && t.programs_dir.join("Crew.lnk").is_file())
}

#[cfg(target_os = "windows")]
pub fn register_windows(t: &WinTarget) -> anyhow::Result<bool> {
    if !is_stale_windows(t) {
        return Ok(false);
    }
    std::fs::create_dir_all(&t.programs_dir)?;
    let link = mslnk::ShellLink::new(&t.exe)?;
    link.create_lnk(t.programs_dir.join("Crew.lnk"))?;
    if let Some(dir) = t.marker.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&t.marker, win_marker_content(&t.exe, &t.version))?;
    Ok(true)
}

#[cfg(target_os = "windows")]
pub fn remove_windows(t: &WinTarget) -> std::io::Result<()> {
    for p in [t.programs_dir.join("Crew.lnk"), t.marker.clone()] {
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }
    Ok(())
}
