use std::path::Path;

use anyhow::Result;

use super::copy::{copy_entry, copy_entry_progress};
use super::progress::{count_files_and_bytes, FileProgress};

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
