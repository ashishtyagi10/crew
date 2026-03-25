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
    /// Already on the latest version.
    UpToDate,
    /// Check failed (network error, etc.) — not fatal.
    Failed(String),
}

/// Check for updates in a background thread.
/// Returns a receiver that will eventually contain the result.
pub fn check_for_updates_async() -> mpsc::Receiver<UpdateStatus> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let status = match check_latest_version() {
            Ok(Some(latest)) => UpdateStatus::Available(latest),
            Ok(None) => UpdateStatus::UpToDate,
            Err(e) => UpdateStatus::Failed(e.to_string()),
        };
        let _ = tx.send(status);
    });

    rx
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
pub fn perform_update() -> Result<self_update::Status> {
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

/// Print the current version.
pub fn print_version() {
    println!("farx {}", cargo_crate_version!());
}
