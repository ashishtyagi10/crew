use std::path::PathBuf;

/// Return the list of candidate config file paths in priority order.
///
/// On macOS `dirs::config_dir()` resolves to `~/Library/Application Support`,
/// but users often expect `~/.config/`, so both are checked.
pub fn candidate_paths() -> Vec<PathBuf> {
    [
        dirs::config_dir().map(|d| d.join("farx").join("config.toml")),
        dirs::home_dir().map(|d| d.join(".config").join("farx").join("config.toml")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Find the first existing config file from the candidate paths.
pub fn find_existing() -> Option<PathBuf> {
    candidate_paths().into_iter().find(|p| p.exists())
}
