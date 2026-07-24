//! Linux app-menu registration: XDG `.desktop` entry + hicolor icons.
//! Split from `appregister.rs` (child module).
use super::*;

// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub struct LinuxTarget {
    /// XDG data dir, normally `~/.local/share`.
    pub data_dir: PathBuf,
    pub exe: PathBuf,
    pub version: String,
}

// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn desktop_entry(exe: &std::path::Path, version: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Crew\n\
         Comment=AI crew terminal\n\
         Exec={exe}\n\
         Icon=crew\n\
         Terminal=false\n\
         Categories=Development;Utility;\n\
         StartupWMClass=crew\n\
         X-Crew-Version={version}\n",
        exe = exe.display(),
    )
}

// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const LINUX_ICON_SIZES: [(u32, &[u8]); 4] = [
    (32, ICON_PNG_32),
    (128, ICON_PNG_128),
    (256, ICON_PNG_256),
    (512, ICON_PNG_512),
];

/// Fresh = the .desktop file matches what we'd write (covers both version
/// and exe path) and every icon size is present.
// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn is_stale_linux(t: &LinuxTarget) -> bool {
    let desktop_ok = std::fs::read_to_string(t.data_dir.join("applications/crew.desktop"))
        .map(|body| body == desktop_entry(&t.exe, &t.version))
        .unwrap_or(false);
    let icons_ok = LINUX_ICON_SIZES.iter().all(|(s, _)| {
        t.data_dir
            .join(format!("icons/hicolor/{s}x{s}/apps/crew.png"))
            .is_file()
    });
    !(desktop_ok && icons_ok)
}

// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn register_linux(t: &LinuxTarget) -> std::io::Result<bool> {
    if !is_stale_linux(t) {
        return Ok(false);
    }
    let apps = t.data_dir.join("applications");
    std::fs::create_dir_all(&apps)?;
    std::fs::write(apps.join("crew.desktop"), desktop_entry(&t.exe, &t.version))?;
    for (s, bytes) in LINUX_ICON_SIZES {
        let dir = t.data_dir.join(format!("icons/hicolor/{s}x{s}/apps"));
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("crew.png"), bytes)?;
    }
    // Best-effort menu/icon cache refresh; absent tools are fine.
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps)
        .status();
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .arg(t.data_dir.join("icons/hicolor"))
        .status();
    Ok(true)
}

// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn remove_linux(t: &LinuxTarget) -> std::io::Result<()> {
    let desktop = t.data_dir.join("applications/crew.desktop");
    if desktop.exists() {
        std::fs::remove_file(&desktop)?;
    }
    for (s, _) in LINUX_ICON_SIZES {
        let icon = t
            .data_dir
            .join(format!("icons/hicolor/{s}x{s}/apps/crew.png"));
        if icon.exists() {
            std::fs::remove_file(&icon)?;
        }
    }
    Ok(())
}
