# Desktop App Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Crew appears in macOS Spotlight/Launchpad, the Windows Start menu, and Linux app menus with its own icon, while keeping single-binary distribution and `/update` intact.

**Architecture:** Icon artwork lives in-repo (`assets/icon/crew.svg`) with committed generated derivatives (`.icns`/`.ico`/PNGs) that the binary embeds via `include_bytes!`. A new `appregister` module in `crew-app` writes a `~/Applications/Crew.app` wrapper (symlink executable), a `crew.desktop` + hicolor icons, or a Start-menu `.lnk` — idempotently, triggered by a new `crew install-app` subcommand and silently on GUI startup from a background thread. Runtime code sets the macOS Dock icon (objc2) and the winit window icon / Linux app_id.

**Tech Stack:** Rust (edition 2021, workspace v0.5.57), winit `=0.30.13`, `image` 0.25 (already a dep, png feature), new deps: `objc2`/`objc2-app-kit`/`objc2-foundation` (macOS only), `mslnk` (Windows only), `winresource` (Windows build-dep only).

**Spec:** `docs/superpowers/specs/2026-07-09-desktop-app-identity-design.md`

## Global Constraints

- Never block the winit main thread: registration triggered from the GUI runs on a `std::thread::spawn` background thread.
- Bundle identifier is exactly `io.github.ashishtyagi10.crew`; user-visible name is exactly `Crew`.
- macOS bundle executable is a **symlink** to the real binary, never a copy (keeps `/update` working). Symlink target rule: prefer `~/.local/bin/crew` if it exists, else `current_exe()`.
- All registration functions are idempotent: unchanged version + unchanged exe path ⇒ second run writes nothing.
- `CREW_NO_APP_INSTALL=1` disables automatic registration (explicit `crew install-app` still works).
- Registration functions take target dirs as parameters so tests write into a tempdir, never the real `$HOME`.
- Generated icon files are committed; no SVG tooling at build/CI time.
- Pre-commit hook runs `cargo fmt` + `cargo check` — run `cargo fmt` before every commit.
- Out of scope: code signing, notarization, .dmg/MSI, file associations, open-at-login.

---

### Task 1: Icon artwork + generation script + committed derivatives

**Files:**
- Create: `assets/icon/crew.svg`
- Create: `scripts/gen-icons.sh`
- Create (generated): `assets/icon/crew.icns`, `assets/icon/crew.ico`, `assets/icon/crew-32.png`, `assets/icon/crew-128.png`, `assets/icon/crew-256.png`, `assets/icon/crew-512.png`

**Interfaces:**
- Produces: the six committed icon files above, embedded by later tasks via `include_bytes!("../../../assets/icon/<file>")` (paths relative to `crates/crew-app/src/`).

- [ ] **Step 1: Write the master SVG**

The mark: a hive cell (hexagon) holding a three-member crew (three connected nodes, lead node in honey), inked on paper — matches the app's paper/ink theme and stays legible at 16 px.

Create `assets/icon/crew.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <!-- paper card -->
  <rect x="64" y="64" width="896" height="896" rx="200" fill="#F2EDE3"/>
  <!-- hive cell -->
  <polygon points="512,148 806,318 806,658 512,828 218,658 218,318"
           fill="none" stroke="#26221B" stroke-width="58" stroke-linejoin="round"/>
  <!-- crew: three members joined -->
  <g stroke="#26221B" stroke-width="34">
    <line x1="512" y1="368" x2="380" y2="598"/>
    <line x1="512" y1="368" x2="644" y2="598"/>
    <line x1="380" y1="598" x2="644" y2="598"/>
  </g>
  <circle cx="512" cy="368" r="86" fill="#E8A13D" stroke="#26221B" stroke-width="30"/>
  <circle cx="380" cy="598" r="86" fill="#F2EDE3" stroke="#26221B" stroke-width="30"/>
  <circle cx="644" cy="598" r="86" fill="#F2EDE3" stroke="#26221B" stroke-width="30"/>
</svg>
```

- [ ] **Step 2: Write the generation script**

Create `scripts/gen-icons.sh` (mode 755):

```sh
#!/bin/sh
# Regenerate the committed icon derivatives from assets/icon/crew.svg.
# Dev-machine only — outputs are committed so CI/builds need no SVG tooling.
# Requires: rsvg-convert (brew install librsvg), ImageMagick `magick`
# (brew install imagemagick); iconutil is macOS built-in.
set -e
cd "$(dirname "$0")/.."
SRC=assets/icon/crew.svg
OUT=assets/icon

for s in 16 32 64 128 256 512 1024; do
    rsvg-convert -w "$s" -h "$s" "$SRC" -o "$OUT/tmp-$s.png"
done

# Linux hicolor sizes + runtime-embedded PNGs (committed).
for s in 32 128 256 512; do
    cp "$OUT/tmp-$s.png" "$OUT/crew-$s.png"
done

# Windows multi-size .ico (committed; embedded by build.rs).
magick "$OUT/tmp-16.png" "$OUT/tmp-32.png" "$OUT/tmp-64.png" \
    "$OUT/tmp-128.png" "$OUT/tmp-256.png" "$OUT/crew.ico"

# macOS .icns (committed; written into Crew.app/Contents/Resources).
if command -v iconutil >/dev/null 2>&1; then
    ISET="$OUT/crew.iconset"
    rm -rf "$ISET" && mkdir "$ISET"
    cp "$OUT/tmp-16.png"   "$ISET/icon_16x16.png"
    cp "$OUT/tmp-32.png"   "$ISET/icon_16x16@2x.png"
    cp "$OUT/tmp-32.png"   "$ISET/icon_32x32.png"
    cp "$OUT/tmp-64.png"   "$ISET/icon_32x32@2x.png"
    cp "$OUT/tmp-128.png"  "$ISET/icon_128x128.png"
    cp "$OUT/tmp-256.png"  "$ISET/icon_128x128@2x.png"
    cp "$OUT/tmp-256.png"  "$ISET/icon_256x256.png"
    cp "$OUT/tmp-512.png"  "$ISET/icon_256x256@2x.png"
    cp "$OUT/tmp-512.png"  "$ISET/icon_512x512.png"
    cp "$OUT/tmp-1024.png" "$ISET/icon_512x512@2x.png"
    iconutil -c icns "$ISET" -o "$OUT/crew.icns"
    rm -rf "$ISET"
else
    echo "warning: iconutil not found — crew.icns NOT regenerated" >&2
fi

rm -f "$OUT"/tmp-*.png
echo "Icons regenerated in $OUT"
```

- [ ] **Step 3: Run the script**

Run: `chmod +x scripts/gen-icons.sh && ./scripts/gen-icons.sh`

If `rsvg-convert` or `magick` is missing: `brew install librsvg imagemagick`, then re-run.

Expected output: `Icons regenerated in assets/icon`

- [ ] **Step 4: Verify the outputs**

Run: `ls -la assets/icon/ && file assets/icon/crew.icns assets/icon/crew.ico assets/icon/crew-512.png`

Expected: 6 generated files present; `file` reports "Mac OS X icon", "MS Windows icon resource" (5 icons), "PNG image data, 512 x 512". Also eyeball the mark: `open assets/icon/crew-512.png` and `qlmanage -p assets/icon/crew.icns` — hexagon + three nodes on paper, honey lead node.

- [ ] **Step 5: Commit**

```bash
git add assets/icon scripts/gen-icons.sh
git commit -m "feat(icon): crew mark SVG + generated icns/ico/png derivatives"
```

---

### Task 2: appregister module — macOS Crew.app bundle

**Files:**
- Create: `crates/crew-app/src/appregister.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod appregister;` after `mod anim;`)

**Interfaces:**
- Consumes: `assets/icon/crew.icns`, `assets/icon/crew-{32,128,256,512}.png` from Task 1.
- Produces (used by Tasks 3–6):
  - `pub const VERSION: &str` (= `env!("CARGO_PKG_VERSION")`)
  - `pub const ICON_PNG_32/ICON_PNG_128/ICON_PNG_256/ICON_PNG_512: &[u8]`
  - `pub fn resolved_exe() -> PathBuf`
  - `pub struct MacTarget { pub apps_dir: PathBuf, pub exe: PathBuf, pub version: String }` (macOS-gated)
  - `pub fn plist(version: &str) -> String`
  - `pub fn register_macos(t: &MacTarget) -> std::io::Result<bool>` — `Ok(true)` if it wrote, `Ok(false)` if fresh
  - `pub fn is_stale_macos(t: &MacTarget) -> bool`
  - `pub fn remove_macos(t: &MacTarget) -> std::io::Result<()>`

- [ ] **Step 1: Create the module skeleton with embedded assets and write the failing plist test**

Create `crates/crew-app/src/appregister.rs`:

```rust
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
}
```

Add `mod appregister;` to `crates/crew-app/src/main.rs` (alphabetically, right after `mod anim;`).

- [ ] **Step 2: Run test to verify it passes (skeleton compiles, plist correct)**

Run: `cargo test -p crew-app appregister -- --nocapture`
Expected: `plist_contains_identity_and_version ... ok` (this step establishes the module; the failing-test cycle is next)

- [ ] **Step 3: Write the failing macOS register/idempotence tests**

Append to the `tests` module in `appregister.rs`:

```rust
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
        let t2 = MacTarget { version: "1.0.1".to_string(), ..t };
        assert!(is_stale_macos(&t2));
        assert!(register_macos(&t2).unwrap());
        assert!(std::fs::read_to_string(contents.join("Info.plist"))
            .unwrap()
            .contains("<string>1.0.1</string>"));

        // Remove deletes the bundle.
        remove_macos(&t2).unwrap();
        assert!(!t2.apps_dir.join("Crew.app").exists());
    }
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p crew-app appregister`
Expected: FAIL to compile — `MacTarget`, `register_macos`, `is_stale_macos`, `remove_macos` not found.

- [ ] **Step 5: Implement the macOS writer**

Add to `appregister.rs` (above the tests module):

```rust
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
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p crew-app appregister`
Expected: 2 passed.

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add crates/crew-app/src/appregister.rs crates/crew-app/src/main.rs
git commit -m "feat(app): appregister module — macOS Crew.app wrapper bundle"
```

---

### Task 3: appregister — Linux .desktop + hicolor icons

**Files:**
- Modify: `crates/crew-app/src/appregister.rs`

**Interfaces:**
- Consumes: `ICON_PNG_*` consts, `VERSION` from Task 2.
- Produces (used by Task 5):
  - `pub struct LinuxTarget { pub data_dir: PathBuf, pub exe: PathBuf, pub version: String }`
  - `pub fn desktop_entry(exe: &Path, version: &str) -> String`
  - `pub fn register_linux(t: &LinuxTarget) -> std::io::Result<bool>`
  - `pub fn is_stale_linux(t: &LinuxTarget) -> bool`
  - `pub fn remove_linux(t: &LinuxTarget) -> std::io::Result<()>`
  - These are NOT `cfg`-gated to linux (pure path/string I/O), so they compile and test on macOS dev machines. Only their *callers* (Task 5) are cfg-gated.

- [ ] **Step 1: Write the failing tests**

Append to the tests module in `appregister.rs`:

```rust
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
        let t2 = LinuxTarget { exe: exe2.clone(), ..t };
        assert!(is_stale_linux(&t2));
        assert!(register_linux(&t2).unwrap());
        let body =
            std::fs::read_to_string(t2.data_dir.join("applications/crew.desktop")).unwrap();
        assert!(body.contains(&format!("Exec={}\n", exe2.display())));

        remove_linux(&t2).unwrap();
        assert!(!t2.data_dir.join("applications/crew.desktop").exists());
        assert!(!t2.data_dir.join("icons/hicolor/512x512/apps/crew.png").exists());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p crew-app appregister`
Expected: FAIL to compile — `LinuxTarget`, `desktop_entry`, `register_linux`… not found.

- [ ] **Step 3: Implement the Linux writer**

Add to `appregister.rs`:

```rust
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
        let icon = t.data_dir.join(format!("icons/hicolor/{s}x{s}/apps/crew.png"));
        if icon.exists() {
            std::fs::remove_file(&icon)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p crew-app appregister`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
cargo fmt
git add crates/crew-app/src/appregister.rs
git commit -m "feat(app): appregister — Linux crew.desktop + hicolor icons"
```

---

### Task 4: appregister — Windows Start-menu .lnk

**Files:**
- Modify: `crates/crew-app/src/appregister.rs`
- Modify: `crates/crew-app/Cargo.toml` (add `mslnk` for Windows targets)

**Interfaces:**
- Consumes: `VERSION` from Task 2.
- Produces (used by Task 5):
  - `pub fn win_marker_content(exe: &Path, version: &str) -> String` (cross-platform, tested everywhere)
  - `pub struct WinTarget { pub programs_dir: PathBuf, pub marker: PathBuf, pub exe: PathBuf, pub version: String }` (windows-gated)
  - `pub fn register_windows(t: &WinTarget) -> anyhow::Result<bool>` / `pub fn is_stale_windows(t: &WinTarget) -> bool` / `pub fn remove_windows(t: &WinTarget) -> std::io::Result<()>` (windows-gated)

- [ ] **Step 1: Add the dependency**

In `crates/crew-app/Cargo.toml`, after the existing `[target.'cfg(unix)'.dependencies]` block, add:

```toml
# Start-menu shortcut writer (pure Rust, no COM) for `install-app`.
[target.'cfg(windows)'.dependencies]
mslnk = "0.1"
```

- [ ] **Step 2: Write the failing marker test**

Append to the tests module in `appregister.rs`:

```rust
    #[test]
    fn win_marker_roundtrip() {
        let m = win_marker_content(std::path::Path::new(r"C:\bin\crew.exe"), "3.0.0");
        assert_eq!(m, r"3.0.0|C:\bin\crew.exe");
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p crew-app appregister`
Expected: FAIL to compile — `win_marker_content` not found.

- [ ] **Step 4: Implement the Windows writer**

Add to `appregister.rs`:

```rust
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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p crew-app appregister`
Expected: 5 passed. (The windows-gated fns don't compile on macOS; the release CI windows build is the compile check — flag any CI failure back.)

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-app/src/appregister.rs crates/crew-app/Cargo.toml Cargo.lock
git commit -m "feat(app): appregister — Windows Start-menu shortcut via mslnk"
```

---

### Task 5: `crew install-app` CLI + auto-register on GUI launch + install.sh hook

**Files:**
- Modify: `crates/crew-app/src/appregister.rs` (top-level dispatch)
- Modify: `crates/crew-app/src/main.rs` (subcommand, before the detach block)
- Modify: `crates/crew-app/src/handler.rs` (background-thread trigger in `run()`)
- Modify: `install.sh`

**Interfaces:**
- Consumes: per-OS register/remove fns from Tasks 2–4.
- Produces:
  - `pub fn register_current(verbose: bool) -> anyhow::Result<()>`
  - `pub fn remove_current() -> anyhow::Result<()>`
  - `pub fn auto_register()` (honors `CREW_NO_APP_INSTALL=1`, swallows errors)
  - CLI: `crew install-app` / `crew install-app --remove`

- [ ] **Step 1: Implement the dispatch layer**

Add to `appregister.rs`:

```rust
/// Register for the current OS using real user dirs. `verbose` prints what
/// happened (CLI path); the auto path stays silent.
pub fn register_current(verbose: bool) -> anyhow::Result<()> {
    let exe = resolved_exe();
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
        let t = MacTarget {
            apps_dir: home.join("Applications"),
            exe,
            version: VERSION.to_string(),
        };
        let wrote = register_macos(&t)?;
        if verbose {
            let app = t.apps_dir.join("Crew.app");
            if wrote {
                println!("Registered {} (Spotlight/Launchpad may take a moment)", app.display());
            } else {
                println!("Already registered: {}", app.display());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let data = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("no XDG data dir"))?;
        let t = LinuxTarget { data_dir: data, exe, version: VERSION.to_string() };
        let wrote = register_linux(&t)?;
        if verbose {
            let entry = t.data_dir.join("applications/crew.desktop");
            if wrote {
                println!("Registered {}", entry.display());
            } else {
                println!("Already registered: {}", entry.display());
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no APPDATA"))?;
        let t = WinTarget {
            programs_dir: appdata.join(r"Microsoft\Windows\Start Menu\Programs"),
            marker: appdata.join(r"crew\app-register"),
            exe,
            version: VERSION.to_string(),
        };
        let wrote = register_windows(&t)?;
        if verbose {
            let lnk = t.programs_dir.join("Crew.lnk");
            if wrote {
                println!("Registered {}", lnk.display());
            } else {
                println!("Already registered: {}", lnk.display());
            }
        }
    }
    Ok(())
}

pub fn remove_current() -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
        let t = MacTarget {
            apps_dir: home.join("Applications"),
            exe: resolved_exe(),
            version: VERSION.to_string(),
        };
        remove_macos(&t)?;
        println!("Removed {}", t.apps_dir.join("Crew.app").display());
    }
    #[cfg(target_os = "linux")]
    {
        let data = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("no XDG data dir"))?;
        let t = LinuxTarget { data_dir: data, exe: resolved_exe(), version: VERSION.to_string() };
        remove_linux(&t)?;
        println!("Removed crew.desktop and icons");
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no APPDATA"))?;
        let t = WinTarget {
            programs_dir: appdata.join(r"Microsoft\Windows\Start Menu\Programs"),
            marker: appdata.join(r"crew\app-register"),
            exe: resolved_exe(),
            version: VERSION.to_string(),
        };
        remove_windows(&t)?;
        println!("Removed Crew.lnk");
    }
    Ok(())
}

/// Silent best-effort registration for GUI startup. Runs on a background
/// thread (never the winit thread); `CREW_NO_APP_INSTALL=1` opts out.
pub fn auto_register() {
    if std::env::var_os("CREW_NO_APP_INSTALL").is_some_and(|v| v == "1") {
        return;
    }
    let _ = register_current(false);
}
```

Note: on macOS `dirs::data_dir()`/`dirs::config_dir()` resolve to Library paths, but the linux/windows blocks are cfg'd out there — each block only compiles on its own OS.

- [ ] **Step 2: Wire the subcommand into `main.rs`**

In `crates/crew-app/src/main.rs`, insert after the `--list-fonts` block and before the detach block:

```rust
    // `crew install-app` — create/refresh the OS app-menu entry (Spotlight /
    // Start menu / .desktop); `--remove` deletes it. Also run automatically
    // by install.sh and silently on GUI startup.
    if std::env::args().skip(1).any(|a| a == "install-app") {
        return if std::env::args().skip(1).any(|a| a == "--remove") {
            appregister::remove_current()
        } else {
            appregister::register_current(true)
        };
    }
```

- [ ] **Step 3: Trigger auto-registration from the GUI path**

In `crates/crew-app/src/handler.rs` `run()`, right after `let event_loop = EventLoop::new()?;` add:

```rust
    // Keep the app-menu entry (Crew.app / .desktop / Start-menu) fresh after
    // updates. Off-thread: registration does file I/O and must never touch
    // the winit thread. CREW_NO_APP_INSTALL=1 opts out (checked inside).
    std::thread::spawn(crate::appregister::auto_register);
```

- [ ] **Step 4: Verify against an isolated HOME**

```bash
FAKE=$(mktemp -d)
HOME="$FAKE" cargo run -p crew-app --bin crew -- install-app
find "$FAKE/Applications/Crew.app" -type f -o -type l
plutil -lint "$FAKE/Applications/Crew.app/Contents/Info.plist"
readlink "$FAKE/Applications/Crew.app/Contents/MacOS/crew"
HOME="$FAKE" cargo run -p crew-app --bin crew -- install-app   # second run
HOME="$FAKE" cargo run -p crew-app --bin crew -- install-app --remove
```

Expected: first run prints `Registered …/Applications/Crew.app`; plist lints OK; readlink prints the dev binary path (no `~/.local/bin/crew` in the fake home); second run prints `Already registered`; `--remove` deletes the bundle.

- [ ] **Step 5: Hook install.sh**

In `install.sh`, after the `echo "Installed …"` line and before the PATH check, add:

```sh
    # Register Crew in the OS app menu (Spotlight / Start menu / .desktop).
    # Best-effort: an older binary without the subcommand must not fail install.
    "${INSTALL_DIR}/${BIN_NAME}" install-app 2>/dev/null || true
```

Verify: `sh -n install.sh` (syntax check) — expected: no output.

- [ ] **Step 6: Run the full test suite and commit**

Run: `cargo test -p crew-app` — expected: all pass.

```bash
cargo fmt
git add crates/crew-app/src/appregister.rs crates/crew-app/src/main.rs crates/crew-app/src/handler.rs install.sh
git commit -m "feat(app): crew install-app subcommand + auto-register on GUI launch"
```

---

### Task 6: Runtime icons — macOS Dock, winit window icon, Linux app_id

**Files:**
- Create: `crates/crew-app/src/dockicon.rs`
- Modify: `crates/crew-app/src/main.rs` (add `mod dockicon;` after `mod dispatch;`)
- Modify: `crates/crew-app/src/handler.rs` (window icon + app_id in `resumed()`, dock icon in `run()`)
- Modify: `crates/crew-app/src/appregister.rs` (`window_icon()` helper)
- Modify: `crates/crew-app/Cargo.toml` (objc2 deps, macOS only)

**Interfaces:**
- Consumes: `ICON_PNG_128`, `ICON_PNG_512` from Task 2.
- Produces: `dockicon::set()` (macOS), `appregister::window_icon() -> Option<winit::window::Icon>` (non-macOS).

- [ ] **Step 1: Add macOS deps**

In `crates/crew-app/Cargo.toml`:

```toml
# Runtime Dock icon (terminal launches have no bundle identity).
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.6"
objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSData"] }
objc2-app-kit = { version = "0.3", default-features = false, features = ["std", "NSApplication", "NSImage", "NSResponder"] }
```

(If the compiler reports a type behind a missing feature — e.g. `NSRunningApplication` — add that feature; the error message names it.)

- [ ] **Step 2: Create `dockicon.rs`**

```rust
//! Sets the Dock icon at runtime. A symlink-executable bundle can lose
//! bundle identity and a plain terminal launch never had one — either way
//! the Dock would show the generic binary icon without this.
#![cfg(target_os = "macos")]

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;

/// Call on the main thread after the event loop (NSApplication) exists.
pub fn set() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(crate::appregister::ICON_PNG_512);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    unsafe { app.setApplicationIconImage(Some(&image)) };
}
```

Add `mod dockicon;` to `main.rs` (alphabetically, after `mod dispatch;`). If method names differ in the resolved objc2-app-kit version (`init_with_data` casing, safety), follow the compiler — the shape stays the same.

- [ ] **Step 3: Add the window-icon helper**

In `appregister.rs`:

```rust
/// Window/taskbar icon for winit (Windows + Linux; macOS uses the Dock).
#[cfg(not(target_os = "macos"))]
pub fn window_icon() -> Option<winit::window::Icon> {
    let img = image::load_from_memory(ICON_PNG_128).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}
```

- [ ] **Step 4: Wire both into `handler.rs`**

In `run()`, right after `let event_loop = EventLoop::new()?;` (next to the Task 5 spawn):

```rust
    #[cfg(target_os = "macos")]
    crate::dockicon::set();
```

In `resumed()`, replace the `let attrs = …` chain with:

```rust
        let attrs = Window::default_attributes()
            .with_title("Crew")
            .with_resizable(true)
            .with_inner_size(LogicalSize::new(w, h));
        // Taskbar/window icon + app_id so Windows/Linux match the menu entry
        // (macOS gets its icon from the bundle + dockicon::set()).
        #[cfg(not(target_os = "macos"))]
        let attrs = attrs.with_window_icon(crate::appregister::window_icon());
        #[cfg(target_os = "linux")]
        let attrs = {
            use winit::platform::wayland::WindowAttributesExtWayland;
            use winit::platform::x11::WindowAttributesExtX11;
            let attrs = WindowAttributesExtWayland::with_name(attrs, "crew", "crew");
            WindowAttributesExtX11::with_name(attrs, "crew", "crew")
        };
```

- [ ] **Step 5: Build and verify the Dock icon live**

Run: `cargo build -p crew-app --bin crew` — expected: clean build.

Live check (uses the repo's GUI verify recipe — see `.claude/skills/verify` for the isolated-HOME launch): launch the dev build, confirm the Dock shows the hexagon-crew mark instead of the generic executable icon. Screenshot for the record.

- [ ] **Step 6: Commit**

```bash
cargo fmt
git add crates/crew-app/src/dockicon.rs crates/crew-app/src/appregister.rs \
        crates/crew-app/src/handler.rs crates/crew-app/src/main.rs \
        crates/crew-app/Cargo.toml Cargo.lock
git commit -m "feat(app): runtime icons — Dock icon on macOS, window icon + app_id elsewhere"
```

---

### Task 7: Windows exe icon via winresource build script

**Files:**
- Create: `crates/crew-app/build.rs`
- Modify: `crates/crew-app/Cargo.toml`

**Interfaces:**
- Consumes: `assets/icon/crew.ico` from Task 1.
- Produces: `crew.exe` with an embedded icon resource (Explorer, taskbar, and the Task 4 `.lnk` all inherit it).

- [ ] **Step 1: Add the build-dependency**

In `crates/crew-app/Cargo.toml` (build-deps are host-cfg'd; the Windows release job builds natively on windows-latest, so host = target there):

```toml
[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
```

- [ ] **Step 2: Create `crates/crew-app/build.rs`**

```rust
//! Embeds the app icon into crew.exe on Windows builds. No-op elsewhere.
fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon/crew.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon/crew.ico");
        res.compile().expect("embed assets/icon/crew.ico");
    }
}
```

- [ ] **Step 3: Verify a non-Windows build is unaffected**

Run: `cargo build -p crew-app --bin crew && cargo test -p crew-app appregister`
Expected: clean build, tests pass. (The Windows path is exercised by the release CI's `x86_64-pc-windows-msvc` job — after the next tag, confirm that job stays green and spot-check the exe icon in the release zip.)

- [ ] **Step 4: Commit**

```bash
cargo fmt
git add crates/crew-app/build.rs crates/crew-app/Cargo.toml
git commit -m "feat(app): embed crew.ico into crew.exe via winresource"
```

---

### Task 8: Docs + end-to-end verification on this Mac

**Files:**
- Modify: `README.md` (install section)
- Modify: `docs/CREW.md` (if it documents CLI flags/commands — add `install-app` alongside them)

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Document the feature**

In `README.md`'s install/usage area, add (adapt placement to the existing structure):

```markdown
### App menu / Spotlight

Crew registers itself in your OS app menu on first GUI launch (Spotlight and
Launchpad on macOS, the Start menu on Windows, the applications menu on
Linux). The entry always launches the installed binary, so `/update` keeps
it current.

- `crew install-app` — create or refresh the entry explicitly
- `crew install-app --remove` — remove it
- `CREW_NO_APP_INSTALL=1` — disable automatic registration
```

Add a matching one-liner wherever `docs/CREW.md` lists CLI modes (`--list-fonts`, `--self-update`, …).

- [ ] **Step 2: Full test suite + fmt**

Run: `cargo fmt && cargo test -p crew-app`
Expected: all tests pass, no fmt diffs.

- [ ] **Step 3: Real end-to-end on this machine**

```bash
cargo run -p crew-app --bin crew -- install-app
ls -la ~/Applications/Crew.app/Contents/MacOS/   # symlink → ~/.local/bin/crew
plutil -lint ~/Applications/Crew.app/Contents/Info.plist
open -a Crew    # launches the installed crew GUI
sleep 20 && mdfind "kMDItemDisplayName == 'Crew'" | grep -q Crew.app && echo SPOTLIGHT-OK
```

Expected: symlink points at `~/.local/bin/crew` (it exists on this machine), plist lints, `open -a Crew` starts the app, `SPOTLIGHT-OK` prints (Spotlight indexing can lag — retry the mdfind once after a minute before treating it as a failure; the icon in Finder/Spotlight should be the crew mark).

- [ ] **Step 4: Commit**

```bash
git add README.md docs/CREW.md
git commit -m "docs: app-menu registration (install-app, opt-out, Spotlight/Start menu)"
```
