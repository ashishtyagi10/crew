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
// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub const ICON_PNG_32: &[u8] = include_bytes!("../../../assets/icon/crew-32.png");
// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub const ICON_PNG_128: &[u8] = include_bytes!("../../../assets/icon/crew-128.png");
// Callers are cfg-gated per-OS; the fns stay un-gated so they compile+test everywhere.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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
                println!(
                    "Registered {} (Spotlight/Launchpad may take a moment)",
                    app.display()
                );
            } else {
                println!("Already registered: {}", app.display());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let data = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("no XDG data dir"))?;
        let t = LinuxTarget {
            data_dir: data,
            exe,
            version: VERSION.to_string(),
        };
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
        let t = LinuxTarget {
            data_dir: data,
            exe: resolved_exe(),
            version: VERSION.to_string(),
        };
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

/// Window/taskbar icon for winit (Windows + Linux; macOS uses the Dock).
#[cfg(not(target_os = "macos"))]
pub fn window_icon() -> Option<winit::window::Icon> {
    let img = image::load_from_memory(ICON_PNG_128).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Silent best-effort registration for GUI startup. Runs on a background
/// thread (never the winit thread); `CREW_NO_APP_INSTALL=1` opts out.
pub fn auto_register() {
    if std::env::var_os("CREW_NO_APP_INSTALL").is_some_and(|v| v == "1") {
        return;
    }
    let _ = register_current(false);
}

// Each OS module's items are re-exported un-gated so they compile+test
// everywhere (mirrors the fns' own `cfg_attr(allow(dead_code))` stance);
// off-OS, only the tests consume the other platforms' items, so the glob
// re-export needs the same allowance.
#[path = "regmac.rs"]
mod regmac;
pub use regmac::*;
#[path = "reglinux.rs"]
mod reglinux;
#[cfg_attr(not(target_os = "linux"), allow(unused_imports))]
pub use reglinux::*;
#[path = "regwin.rs"]
mod regwin;
#[cfg_attr(not(target_os = "windows"), allow(unused_imports))]
pub use regwin::*;

#[cfg(test)]
#[path = "appregister_tests.rs"]
mod tests;
