use std::path::Path;

use anyhow::Result;

/// Create a directory
pub fn create_directory(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

/// Rename/move a file
pub fn rename_entry(from: &Path, to: &Path) -> Result<()> {
    std::fs::rename(from, to)?;
    Ok(())
}

/// Create a symbolic link at `link_path` pointing to `target`.
pub fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)?;
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link_path)?;
        } else {
            std::os::windows::fs::symlink_file(target, link_path)?;
        }
    }
    Ok(())
}
