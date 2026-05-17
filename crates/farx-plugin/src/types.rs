use std::path::PathBuf;

/// A registered plugin command.
#[derive(Debug, Clone)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    pub plugin_file: String,
}

/// Result of executing a plugin command.
#[derive(Debug, Clone)]
pub enum PluginResult {
    /// Display a message to the user.
    Message(String),
    /// Execute a shell command and show output.
    Shell(String),
    /// No visible output.
    None,
}

pub(crate) fn lua_err(e: mlua::Error) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

pub fn plugin_directory() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("farx")
        .join("plugins")
}
