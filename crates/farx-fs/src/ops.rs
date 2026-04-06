use std::path::Path;

use anyhow::Result;

/// Copy a file or directory recursively to destination
pub fn copy_entry(source: &Path, dest_dir: &Path) -> Result<()> {
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("No file name"))?;
    let dest = dest_dir.join(file_name);

    if source.is_dir() {
        copy_dir_recursive(source, &dest)?;
    } else {
        std::fs::copy(source, &dest)?;
    }
    Ok(())
}

/// Copy directory recursively
fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let dest_path = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

/// Move a file or directory to destination
pub fn move_entry(source: &Path, dest_dir: &Path) -> Result<()> {
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("No file name"))?;
    let dest = dest_dir.join(file_name);

    // Try rename first (fast, same filesystem)
    match std::fs::rename(source, &dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Cross-filesystem: copy then delete
            copy_entry(source, dest_dir)?;
            if source.is_dir() {
                std::fs::remove_dir_all(source)?;
            } else {
                std::fs::remove_file(source)?;
            }
            Ok(())
        }
    }
}

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
