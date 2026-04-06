use anyhow::Result;
use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use semver::Version;
use std::sync::mpsc;
use std::thread;

/// GitHub repository owner — change this to your GitHub username/org.
const REPO_OWNER: &str = "ashishtyagi10";
/// GitHub repository name.
const REPO_NAME: &str = "farx";

/// Result of a background update check.
pub enum UpdateStatus {
    /// A newer version is available.
    Available(String),
    /// Auto-updated to a new version (restart needed).
    Updated(String),
    /// Already on the latest version.
    UpToDate,
    /// Check failed (network error, etc.) — not fatal.
    Failed(String),
}

/// Check for updates in a background thread and auto-apply if possible.
/// Returns a receiver that will eventually contain the result.
pub fn check_and_auto_update_async() -> mpsc::Receiver<UpdateStatus> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let status = match check_latest_version() {
            Ok(Some(latest)) => {
                // Try to auto-update (works when binary is user-writable)
                match try_auto_update() {
                    Ok(self_update::Status::Updated(v)) => UpdateStatus::Updated(v),
                    _ => UpdateStatus::Available(latest),
                }
            }
            Ok(None) => UpdateStatus::UpToDate,
            Err(e) => UpdateStatus::Failed(e.to_string()),
        };
        let _ = tx.send(status);
    });

    rx
}

/// Attempt to update in-place. Succeeds without sudo if binary is in a
/// user-writable location (e.g. ~/.local/bin).
fn try_auto_update() -> Result<self_update::Status> {
    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("farx")
        .identifier("farx")
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .show_download_progress(false)
        .build()?
        .update()?;
    Ok(status)
}

/// Check if a newer version exists on GitHub Releases.
/// Returns `Ok(Some(version))` if newer, `Ok(None)` if up to date.
fn check_latest_version() -> Result<Option<String>> {
    let current = cargo_crate_version!();
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;

    if let Some(latest) = releases.first() {
        let latest_ver = latest.version.trim_start_matches('v');
        let current_ver = Version::parse(current)?;
        if let Ok(remote_ver) = Version::parse(latest_ver) {
            if remote_ver > current_ver {
                return Ok(Some(remote_ver.to_string()));
            }
        }
    }

    Ok(None)
}

/// Perform the actual update: download and replace the current binary.
/// This should be called outside the TUI (terminal restored first).
/// If the current binary is not writable (e.g. /usr/local/bin), it will
/// attempt to install to ~/.local/bin instead.
pub fn perform_update() -> Result<self_update::Status> {
    // First try updating in-place
    match do_update() {
        Ok(status) => Ok(status),
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Permission denied") || err_str.contains("permission denied") {
                // Binary is in a root-owned dir — try installing to ~/.local/bin
                eprintln!();
                eprintln!(
                    "Permission denied updating in-place. \
                     Attempting install to ~/.local/bin ..."
                );
                install_to_local_bin()
            } else {
                Err(e)
            }
        }
    }
}

fn do_update() -> Result<self_update::Status> {
    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("farx")
        .identifier("farx")
        .current_version(cargo_crate_version!())
        .show_download_progress(true)
        .no_confirm(true)
        .build()?
        .update()?;
    Ok(status)
}

/// Fallback: download the latest release and install to ~/.local/bin.
fn install_to_local_bin() -> Result<self_update::Status> {
    let local_bin = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(".local")
        .join("bin");

    std::fs::create_dir_all(&local_bin)?;
    let dest = local_bin.join("farx");

    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("farx")
        .identifier("farx")
        .current_version(cargo_crate_version!())
        .show_download_progress(true)
        .no_confirm(true)
        .bin_install_path(&local_bin)
        .build()?
        .update()?;

    eprintln!();
    eprintln!("Installed to {}", dest.display());

    // Check if ~/.local/bin is in PATH
    if let Ok(path) = std::env::var("PATH") {
        let local_str = local_bin.to_string_lossy();
        if !path.split(':').any(|p| p == local_str.as_ref()) {
            eprintln!();
            eprintln!("NOTE: {} is not in your PATH. Add it:", local_bin.display());
            eprintln!(
                "  echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc && source ~/.zshrc",
                local_bin.display()
            );
        }
    }

    Ok(status)
}

/// Print the current version.
pub fn print_version() {
    println!("farx {}", cargo_crate_version!());
}
