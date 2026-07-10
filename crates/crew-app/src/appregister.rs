//! Desktop app registration: a Crew.app wrapper bundle (macOS), a
//! crew.desktop entry (Linux), and a Start-menu shortcut (Windows), so Crew
//! shows up in Spotlight/Launchpad/Start-menu with its own icon.
//!
//! Written by `crew install-app` and refreshed silently on GUI startup (off
//! the winit thread). All writers are idempotent: same version + same exe
//! path means the second run touches nothing. The macOS bundle executable is
//! a symlink to the real binary — never a copy — so `/update` replacing
//! `~/.local/bin/crew` updates the app too.
use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// `const` (not `static`) so const tables like LINUX_ICON_SIZES can refer
// to them (constants cannot refer to statics).
pub const ICON_PNG_32: &[u8] = include_bytes!("../../../assets/icon/crew-32.png");
pub const ICON_PNG_128: &[u8] = include_bytes!("../../../assets/icon/crew-128.png");
pub const ICON_PNG_256: &[u8] = include_bytes!("../../../assets/icon/crew-256.png");
pub const ICON_PNG_512: &[u8] = include_bytes!("../../../assets/icon/crew-512.png");
#[cfg(target_os = "macos")]
pub const ICON_ICNS: &[u8] = include_bytes!("../../../assets/icon/crew.icns");

/// The binary the menu entry should launch: prefer the stable installed
/// location so a dev-build registration doesn't point the bundle at a
/// `target/` path that later disappears.
pub fn resolved_exe() -> PathBuf {
    if let Some(installed) = dirs::home_dir().map(|h| h.join(".local/bin/crew")) {
        if installed.is_file() {
            return installed;
        }
    }
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("crew"))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plist_contains_identity_and_version() {
        let p = plist("9.9.9");
        assert!(p.contains("<string>io.github.ashishtyagi10.crew</string>"));
        assert!(p.contains("<key>CFBundleExecutable</key><string>crew</string>"));
        assert!(p.contains("<key>CFBundleIconFile</key><string>crew</string>"));
        assert!(p.contains("<string>9.9.9</string>"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn register_macos_writes_bundle_then_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("crew-bin");
        std::fs::write(&exe, b"fake").unwrap();
        let t = MacTarget {
            apps_dir: tmp.path().join("Applications"),
            exe: exe.clone(),
            version: "1.0.0".to_string(),
        };

        // First run writes the full bundle.
        assert!(register_macos(&t).unwrap());
        let contents = t.apps_dir.join("Crew.app/Contents");
        assert!(contents.join("Info.plist").is_file());
        assert!(contents.join("Resources/crew.icns").is_file());
        let link = contents.join("MacOS/crew");
        assert_eq!(std::fs::read_link(&link).unwrap(), exe);

        // Second run: fresh, writes nothing.
        assert!(!is_stale_macos(&t));
        assert!(!register_macos(&t).unwrap());

        // Version bump → stale → rewrite.
        let t2 = MacTarget {
            version: "1.0.1".to_string(),
            ..t
        };
        assert!(is_stale_macos(&t2));
        assert!(register_macos(&t2).unwrap());
        assert!(std::fs::read_to_string(contents.join("Info.plist"))
            .unwrap()
            .contains("<string>1.0.1</string>"));

        // Remove deletes the bundle.
        remove_macos(&t2).unwrap();
        assert!(!t2.apps_dir.join("Crew.app").exists());
    }
}
