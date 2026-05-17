use std::path::Path;

use anyhow::Result;

use super::progress::{count_files_and_bytes, FileProgress};

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

pub(super) fn copy_entry_progress(
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
