use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub path: PathBuf,
}

/// Load bookmarks from the config directory.
pub fn load_bookmarks() -> Vec<Bookmark> {
    let path = bookmarks_file_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Save bookmarks to the config directory.
pub fn save_bookmarks(bookmarks: &[Bookmark]) {
    let path = bookmarks_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(bookmarks) {
        let _ = std::fs::write(&path, json);
    }
}

fn bookmarks_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("farx")
        .join("bookmarks.json")
}
