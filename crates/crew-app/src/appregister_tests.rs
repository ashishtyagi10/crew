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
