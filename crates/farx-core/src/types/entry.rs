use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Local>>,
    pub extension: Option<String>,
    pub readonly: bool,
    /// Unix permission mode bits (e.g. 0o755). None on non-Unix platforms.
    pub mode: Option<u32>,
}
