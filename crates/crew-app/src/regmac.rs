//! macOS app-menu registration: the `~/Applications/Crew.app` shim bundle.
//! Split from `appregister.rs` (child module).
use super::*;

pub fn plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Crew</string>
    <key>CFBundleDisplayName</key><string>Crew</string>
    <key>CFBundleIdentifier</key><string>io.github.ashishtyagi10.crew</string>
    <key>CFBundleExecutable</key><string>crew</string>
    <key>CFBundleIconFile</key><string>crew</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>{version}</string>
    <key>CFBundleVersion</key><string>{version}</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#
    )
}

#[cfg(target_os = "macos")]
pub struct MacTarget {
    /// Where the bundle goes, normally `~/Applications`.
    pub apps_dir: PathBuf,
    /// Symlink target — the real binary.
    pub exe: PathBuf,
    pub version: String,
}

/// Fresh = plist carries the current version, the executable symlink points
/// at the current exe, and the icon is present.
#[cfg(target_os = "macos")]
pub fn is_stale_macos(t: &MacTarget) -> bool {
    let contents = t.apps_dir.join("Crew.app/Contents");
    let plist_ok = std::fs::read_to_string(contents.join("Info.plist"))
        .map(|p| p.contains(&format!("<string>{}</string>", t.version)))
        .unwrap_or(false);
    let link_ok = std::fs::read_link(contents.join("MacOS/crew"))
        .map(|target| target == t.exe)
        .unwrap_or(false);
    let icns_ok = contents.join("Resources/crew.icns").is_file();
    !(plist_ok && link_ok && icns_ok)
}

#[cfg(target_os = "macos")]
pub fn register_macos(t: &MacTarget) -> std::io::Result<bool> {
    if !is_stale_macos(t) {
        return Ok(false);
    }
    let contents = t.apps_dir.join("Crew.app/Contents");
    std::fs::create_dir_all(contents.join("MacOS"))?;
    std::fs::create_dir_all(contents.join("Resources"))?;
    std::fs::write(contents.join("Info.plist"), plist(&t.version))?;
    std::fs::write(contents.join("Resources/crew.icns"), ICON_ICNS)?;
    let link = contents.join("MacOS/crew");
    // Recreate unconditionally: symlink() fails if the path exists.
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&t.exe, &link)?;
    Ok(true)
}

#[cfg(target_os = "macos")]
pub fn remove_macos(t: &MacTarget) -> std::io::Result<()> {
    let app = t.apps_dir.join("Crew.app");
    if app.exists() {
        std::fs::remove_dir_all(&app)?;
    }
    Ok(())
}
