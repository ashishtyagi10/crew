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

/// Progress update for copy/move operations.
#[derive(Debug, Clone)]
pub struct FileProgress {
    /// Current file being processed.
    pub current_file: String,
    /// Number of files completed so far.
    pub files_done: usize,
    /// Total number of files.
    pub files_total: usize,
    /// Total bytes copied so far.
    pub bytes_done: u64,
    /// Total bytes to copy.
    pub bytes_total: u64,
    /// Whether the operation is complete.
    pub finished: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Count total files and bytes in a path (recursively for directories).
fn count_files_and_bytes(path: &Path) -> (usize, u64) {
    if path.is_dir() {
        let mut count = 0usize;
        let mut bytes = 0u64;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let (c, b) = count_files_and_bytes(&entry.path());
                count += c;
                bytes += b;
            }
        }
        (count, bytes)
    } else {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        (1, size)
    }
}

/// Copy entries with progress reporting via a channel.
pub fn copy_entries_with_progress(
    sources: Vec<std::path::PathBuf>,
    dest_dir: std::path::PathBuf,
    tx: std::sync::mpsc::Sender<FileProgress>,
) {
    // Count total files and bytes
    let mut files_total = 0usize;
    let mut bytes_total = 0u64;
    for src in &sources {
        let (c, b) = count_files_and_bytes(src);
        files_total += c;
        bytes_total += b;
    }

    let mut files_done = 0usize;
    let mut bytes_done = 0u64;

    for src in &sources {
        let result = copy_entry_progress(
            src,
            &dest_dir,
            &tx,
            &mut files_done,
            &mut bytes_done,
            files_total,
            bytes_total,
        );
        if let Err(e) = result {
            let _ = tx.send(FileProgress {
                current_file: src.display().to_string(),
                files_done,
                files_total,
                bytes_done,
                bytes_total,
                finished: true,
                error: Some(e.to_string()),
            });
            return;
        }
    }

    let _ = tx.send(FileProgress {
        current_file: String::new(),
        files_done,
        files_total,
        bytes_done,
        bytes_total,
        finished: true,
        error: None,
    });
}

fn copy_entry_progress(
    source: &Path,
    dest_dir: &Path,
    tx: &std::sync::mpsc::Sender<FileProgress>,
    files_done: &mut usize,
    bytes_done: &mut u64,
    files_total: usize,
    bytes_total: u64,
) -> Result<()> {
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("No file name"))?;
    let dest = dest_dir.join(file_name);

    if source.is_dir() {
        std::fs::create_dir_all(&dest)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                copy_entry_progress(
                    &entry.path(),
                    &dest,
                    tx,
                    files_done,
                    bytes_done,
                    files_total,
                    bytes_total,
                )?;
            } else {
                let name = entry.file_name().to_string_lossy().to_string();
                let _ = tx.send(FileProgress {
                    current_file: name,
                    files_done: *files_done,
                    files_total,
                    bytes_done: *bytes_done,
                    bytes_total,
                    finished: false,
                    error: None,
                });
                let size = std::fs::copy(entry.path(), dest.join(entry.file_name()))?;
                *files_done += 1;
                *bytes_done += size;
            }
        }
    } else {
        let name = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let _ = tx.send(FileProgress {
            current_file: name,
            files_done: *files_done,
            files_total,
            bytes_done: *bytes_done,
            bytes_total,
            finished: false,
            error: None,
        });
        let size = std::fs::copy(source, &dest)?;
        *files_done += 1;
        *bytes_done += size;
    }
    Ok(())
}

/// Move entries with progress reporting via a channel.
pub fn move_entries_with_progress(
    sources: Vec<std::path::PathBuf>,
    dest_dir: std::path::PathBuf,
    tx: std::sync::mpsc::Sender<FileProgress>,
) {
    let mut files_total = 0usize;
    let mut bytes_total = 0u64;
    for src in &sources {
        let (c, b) = count_files_and_bytes(src);
        files_total += c;
        bytes_total += b;
    }

    let mut files_done = 0usize;
    let mut bytes_done = 0u64;

    for src in &sources {
        let file_name = match src.file_name() {
            Some(n) => n,
            None => continue,
        };
        let dest = dest_dir.join(file_name);
        let name = file_name.to_string_lossy().to_string();

        let _ = tx.send(FileProgress {
            current_file: name.clone(),
            files_done,
            files_total,
            bytes_done,
            bytes_total,
            finished: false,
            error: None,
        });

        // Try rename first (fast, same filesystem)
        match std::fs::rename(src, &dest) {
            Ok(()) => {
                let (c, b) = count_files_and_bytes(&dest);
                files_done += c;
                bytes_done += b;
            }
            Err(_) => {
                // Cross-filesystem: copy then delete
                let result = copy_entry_progress(
                    src,
                    &dest_dir,
                    &tx,
                    &mut files_done,
                    &mut bytes_done,
                    files_total,
                    bytes_total,
                );
                if let Err(e) = result {
                    let _ = tx.send(FileProgress {
                        current_file: name,
                        files_done,
                        files_total,
                        bytes_done,
                        bytes_total,
                        finished: true,
                        error: Some(e.to_string()),
                    });
                    return;
                }
                if src.is_dir() {
                    let _ = std::fs::remove_dir_all(src);
                } else {
                    let _ = std::fs::remove_file(src);
                }
            }
        }
    }

    let _ = tx.send(FileProgress {
        current_file: String::new(),
        files_done,
        files_total,
        bytes_done,
        bytes_total,
        finished: true,
        error: None,
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn copy_entry_copies_file_into_destination_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dst_dir = tmp.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src = src_dir.join("file.txt");
        std::fs::write(&src, "hello").unwrap();

        copy_entry(&src, &dst_dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("file.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn copy_entry_copies_directory_recursively() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("srcdir");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(src.join("nested")).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("nested").join("a.txt"), "a").unwrap();

        copy_entry(&src, &dst).unwrap();
        assert!(dst.join("srcdir").join("nested").join("a.txt").exists());
    }

    #[test]
    fn move_entry_moves_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        let dst_dir = tmp.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src = src_dir.join("move.txt");
        std::fs::write(&src, "mv").unwrap();

        move_entry(&src, &dst_dir).unwrap();
        assert!(!src.exists());
        assert_eq!(
            std::fs::read_to_string(dst_dir.join("move.txt")).unwrap(),
            "mv"
        );
    }

    #[test]
    fn copy_entries_with_progress_sends_finished_message() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("a.txt");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(&src, "progress").unwrap();
        let (tx, rx) = mpsc::channel();

        copy_entries_with_progress(vec![src], dst, tx);
        let events: Vec<FileProgress> = rx.iter().collect();
        let last = events.last().unwrap();
        assert!(last.finished);
        assert!(last.error.is_none());
        assert_eq!(last.files_done, 1);
    }

    #[test]
    fn move_entries_with_progress_sends_finished_message() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("b.txt");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(&src, "progress").unwrap();
        let (tx, rx) = mpsc::channel();

        move_entries_with_progress(vec![src], dst, tx);
        let events: Vec<FileProgress> = rx.iter().collect();
        let last = events.last().unwrap();
        assert!(last.finished);
        assert!(last.error.is_none());
        assert_eq!(last.files_done, 1);
    }
}
