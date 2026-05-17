use std::path::Path;

use anyhow::Result;
use farx_core::FileEntry;

/// Read a directory and return a list of FileEntry.
/// If show_hidden is false, hidden files are excluded.
/// On Unix, hidden = starts with dot.
/// On Windows, hidden = has FILE_ATTRIBUTE_HIDDEN.
/// The ".." parent entry is always included at the top if not at root.
pub fn read_directory(path: &Path, show_hidden: bool) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    // Add parent directory entry (..) if not at filesystem root
    if let Some(parent) = path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_path_buf(),
            is_dir: true,
            is_symlink: false,
            is_hidden: false,
            size: 0,
            modified: None,
            extension: None,
            readonly: false,
            mode: None,
        });
    }

    // Read directory entries
    let read_dir = std::fs::read_dir(path)?;
    for entry in read_dir {
        let entry = entry?;
        let metadata = entry.metadata()?; // follows symlinks
        let symlink_meta = entry.path().symlink_metadata().ok();
        let name = entry.file_name().to_string_lossy().to_string();

        let is_hidden = is_hidden_file(&name, &entry.path());
        if !show_hidden && is_hidden {
            continue;
        }

        let is_symlink = symlink_meta.map(|m| m.is_symlink()).unwrap_or(false);
        let modified = metadata
            .modified()
            .ok()
            .map(chrono::DateTime::<chrono::Local>::from);
        let extension = if metadata.is_file() {
            entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_string())
        } else {
            None
        };
        let readonly = metadata.permissions().readonly();

        entries.push(FileEntry {
            name,
            path: entry.path(),
            is_dir: metadata.is_dir(),
            is_symlink,
            is_hidden,
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified,
            extension,
            readonly,
            mode: {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    Some(metadata.permissions().mode())
                }
                #[cfg(not(unix))]
                {
                    None
                }
            },
        });
    }

    Ok(entries)
}

#[cfg(unix)]
fn is_hidden_file(name: &str, _path: &Path) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_directory_hides_dotfiles_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("visible.txt"), "v").unwrap();
        std::fs::write(tmp.path().join(".hidden.txt"), "h").unwrap();

        let entries = read_directory(tmp.path(), false).unwrap();
        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        assert!(names.contains(&"visible.txt".to_string()));
        assert!(!names.contains(&".hidden.txt".to_string()));
    }

    #[test]
    fn read_directory_includes_hidden_when_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".hidden.txt"), "h").unwrap();

        let entries = read_directory(tmp.path(), true).unwrap();
        let hidden = entries.iter().find(|e| e.name == ".hidden.txt").unwrap();
        assert!(hidden.is_hidden);
        assert_eq!(hidden.extension.as_deref(), Some("txt"));
    }
}

#[cfg(windows)]
fn is_hidden_file(_name: &str, path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    if let Ok(meta) = std::fs::metadata(path) {
        meta.file_attributes() & 0x2 != 0 // FILE_ATTRIBUTE_HIDDEN
    } else {
        false
    }
}

#[cfg(not(any(unix, windows)))]
fn is_hidden_file(name: &str, _path: &Path) -> bool {
    name.starts_with('.')
}
