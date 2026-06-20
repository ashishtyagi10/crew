//! GitHub release-list check used by the background updater.

use anyhow::Result;
use self_update::cargo_crate_version;
use semver::Version;
use std::sync::mpsc;
use std::thread;

use super::{REPO_NAME, REPO_OWNER};

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

/// Check for updates in a background thread (check only, never auto-apply).
pub fn check_and_auto_update_async() -> mpsc::Receiver<UpdateStatus> {
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
