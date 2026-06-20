//! Pick the right release asset for this platform and unpack the `farx`
//! binary from the downloaded archive.

use anyhow::Result;
use self_update::update::ReleaseAsset;
use std::path::Path;

/// Determine the right asset for this platform.
pub(super) fn select_asset(assets: &[ReleaseAsset]) -> Result<ReleaseAsset> {
    let target = self_update::get_target();
    let asset = assets
        .iter()
        .find(|a| a.name.contains(target))
        .or_else(|| {
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
            assets
                .iter()
                .find(|a| a.name.contains(os) && a.name.contains(arch))
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No release asset found for target '{}'. Available: {}",
                target,
                assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;
    Ok(asset.clone())
}

/// Extract the `farx` binary from the downloaded archive into `tmp_bin`.
pub(super) fn extract_binary(asset_name: &str, tmp_archive: &Path, tmp_bin: &Path) -> Result<()> {
    if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        let file = std::fs::File::open(tmp_archive)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            if path.file_name().map(|n| n == "farx").unwrap_or(false) {
                entry.unpack(tmp_bin)?;
                break;
            }
        }
    } else if asset_name.ends_with(".zip") {
        let file = std::fs::File::open(tmp_archive)?;
        let mut archive = zip::ZipArchive::new(file)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.name().ends_with("farx") || entry.name().ends_with("farx.exe") {
                let mut out = std::fs::File::create(tmp_bin)?;
                std::io::copy(&mut entry, &mut out)?;
                break;
            }
        }
    } else {
        std::fs::copy(tmp_archive, tmp_bin)?;
    }
    Ok(())
}

/// Mark the temporary binary executable on Unix.
pub(super) fn make_executable(tmp_bin: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp_bin, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    {
        let _ = tmp_bin;
    }
    Ok(())
}
