use anyhow::Result;
use self_update::cargo_crate_version;
use semver::Version;
use std::sync::mpsc;
use std::thread;

/// GitHub repository owner.
const REPO_OWNER: &str = "ashishtyagi10";
/// GitHub repository name.
const REPO_NAME: &str = "farx";

/// Result of a background update check.
#[allow(dead_code)]
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

/// Perform the actual update: download the latest release and install
/// to ~/.local/bin. Never requires sudo — works entirely in user space.
pub fn perform_update() -> Result<self_update::Status> {
    let current = cargo_crate_version!();

    // Fetch release list to find latest
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;

    let latest = releases
        .first()
        .ok_or_else(|| anyhow::anyhow!("No releases found"))?;

    let latest_ver = latest.version.trim_start_matches('v');
    let current_ver = Version::parse(current)?;
    let remote_ver = Version::parse(latest_ver)?;

    if remote_ver <= current_ver {
        return Ok(self_update::Status::UpToDate(current.to_string()));
    }

    // Determine the right asset for this platform
    let target = self_update::get_target();
    let asset = latest
        .assets
        .iter()
        .find(|a| a.name.contains(&target))
        .or_else(|| {
            // Fallback: look for common platform substrings
            let os = if cfg!(target_os = "macos") {
                "apple"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "windows"
            };
            let arch = if cfg!(target_arch = "aarch64") {
                "aarch64"
            } else {
                "x86_64"
            };
            latest
                .assets
                .iter()
                .find(|a| a.name.contains(os) && a.name.contains(arch))
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No release asset found for target '{}'. Available: {}",
                target,
                latest
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    // Download to a temp file
    println!("Downloading {}...", asset.name);
    let tmp_dir = tempfile::tempdir()?;
    let tmp_archive = tmp_dir.path().join(&asset.name);

    let response = reqwest::blocking::get(&asset.download_url)?;
    let bytes = response.bytes()?;
    std::fs::write(&tmp_archive, &bytes)?;

    // Extract the binary from the archive
    let tmp_bin = tmp_dir.path().join("farx");
    if asset.name.ends_with(".tar.gz") || asset.name.ends_with(".tgz") {
        let file = std::fs::File::open(&tmp_archive)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            if path.file_name().map(|n| n == "farx").unwrap_or(false) {
                entry.unpack(&tmp_bin)?;
                break;
            }
        }
    } else if asset.name.ends_with(".zip") {
        let file = std::fs::File::open(&tmp_archive)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.name().ends_with("farx") || entry.name().ends_with("farx.exe") {
                let mut out = std::fs::File::create(&tmp_bin)?;
                std::io::copy(&mut entry, &mut out)?;
                break;
            }
        }
    } else {
        // Assume raw binary
        std::fs::copy(&tmp_archive, &tmp_bin)?;
    }

    if !tmp_bin.exists() {
        anyhow::bail!("Could not find 'farx' binary in the release archive");
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_bin, std::fs::Permissions::from_mode(0o755))?;
    }

    // Install to ~/.local/bin
    let local_bin = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join(".local")
        .join("bin");
    std::fs::create_dir_all(&local_bin)?;

    let dest = local_bin.join("farx");
    std::fs::copy(&tmp_bin, &dest)?;

    println!("Installed to {}", dest.display());

    // Check if ~/.local/bin is in PATH
    if let Ok(path) = std::env::var("PATH") {
        let local_str = local_bin.to_string_lossy();
        if !path.split(':').any(|p| p == local_str.as_ref()) {
            println!();
            println!("NOTE: {} is not in your PATH. Add it:", local_bin.display());
            println!(
                "  echo 'export PATH=\"{}:$PATH\"' >> ~/.zshrc && source ~/.zshrc",
                local_bin.display()
            );
        }
    }

    // Warn if a root-owned copy shadows ~/.local/bin
    if let Ok(current_exe) = std::env::current_exe() {
        if !current_exe.starts_with(&local_bin) {
            println!();
            println!(
                "NOTE: You're running farx from {} which may shadow the update.",
                current_exe.display()
            );
            println!(
                "      Remove the old copy: sudo rm {}",
                current_exe.display()
            );
        }
    }

    Ok(self_update::Status::Updated(remote_ver.to_string()))
}

/// Print the current version.
pub fn print_version() {
    println!("farx {}", cargo_crate_version!());
}
