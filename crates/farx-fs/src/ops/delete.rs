use std::path::Path;

use anyhow::Result;

/// Delete a file or directory.
/// If `use_trash` is true, moves to the system trash/recycle bin.
pub fn delete_entry(path: &Path, use_trash: bool) -> Result<()> {
    if use_trash {
        trash::delete(path).map_err(|e| anyhow::anyhow!("Trash: {}", e))
    } else {
        delete_permanent(path)
    }
}

fn delete_permanent(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
