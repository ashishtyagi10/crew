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

pub struct LinuxTarget {
    /// XDG data dir, normally `~/.local/share`.
    pub data_dir: PathBuf,
    pub exe: PathBuf,
    pub version: String,
}

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

const LINUX_ICON_SIZES: [(u32, &[u8]); 4] = [
    (32, ICON_PNG_32),
    (128, ICON_PNG_128),
    (256, ICON_PNG_256),
    (512, ICON_PNG_512),
];

/// Fresh = the .desktop file matches what we'd write (covers both version
/// and exe path) and every icon size is present.
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

/// Contents of the sidecar staleness marker for the Start-menu shortcut
/// (.lnk files aren't cheaply parseable, so we track what we wrote).
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

    #[test]
    fn desktop_entry_fields() {
        let d = desktop_entry(std::path::Path::new("/home/u/.local/bin/crew"), "2.0.0");
        assert!(d.starts_with("[Desktop Entry]\n"));
        assert!(d.contains("Name=Crew\n"));
        assert!(d.contains("Exec=/home/u/.local/bin/crew\n"));
        assert!(d.contains("Icon=crew\n"));
        assert!(d.contains("Terminal=false\n"));
        assert!(d.contains("StartupWMClass=crew\n"));
        assert!(d.contains("X-Crew-Version=2.0.0\n"));
    }

    #[test]
    fn register_linux_writes_then_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("crew-bin");
        std::fs::write(&exe, b"fake").unwrap();
        let t = LinuxTarget {
            data_dir: tmp.path().join("share"),
            exe: exe.clone(),
            version: "1.0.0".to_string(),
        };

        assert!(register_linux(&t).unwrap());
        assert!(t.data_dir.join("applications/crew.desktop").is_file());
        for s in [32u32, 128, 256, 512] {
            assert!(t
                .data_dir
                .join(format!("icons/hicolor/{s}x{s}/apps/crew.png"))
                .is_file());
        }

        assert!(!is_stale_linux(&t));
        assert!(!register_linux(&t).unwrap());

        // Different exe path → stale → rewrite with the new Exec.
        let exe2 = tmp.path().join("crew-bin2");
        std::fs::write(&exe2, b"fake").unwrap();
        let t2 = LinuxTarget {
            exe: exe2.clone(),
            ..t
        };
        assert!(is_stale_linux(&t2));
        assert!(register_linux(&t2).unwrap());
        let body = std::fs::read_to_string(t2.data_dir.join("applications/crew.desktop")).unwrap();
        assert!(body.contains(&format!("Exec={}\n", exe2.display())));

        remove_linux(&t2).unwrap();
        assert!(!t2.data_dir.join("applications/crew.desktop").exists());
        assert!(!t2
            .data_dir
            .join("icons/hicolor/512x512/apps/crew.png")
            .exists());
    }

    #[test]
    fn win_marker_roundtrip() {
        let m = win_marker_content(std::path::Path::new(r"C:\bin\crew.exe"), "3.0.0");
        assert_eq!(m, r"3.0.0|C:\bin\crew.exe");
    }
}
