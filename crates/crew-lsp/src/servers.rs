//! Which binary serves which language, and whether it is on this machine.
//!
//! A built-in table covers the four ecosystems crew's viewer already
//! colours; `~/.config/crew/lsp.json` overrides or extends it in the shape
//! `{"servers": {"rust": {"command": "…", "args": […]}}}` — the same
//! file-per-user, merged-on-name pattern `mcp.json` uses. Nothing here starts
//! a process: [`available`] is a PATH lookup, so the viewer can ask it on
//! every open without paying for a spawn.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// How to launch one language server.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Server {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

impl Server {
    fn new(command: &str, args: &[&str]) -> Self {
        Self {
            command: command.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// The language id for a file, by extension — the id the server is told in
/// `didOpen`, so it must be the protocol's spelling, not the viewer's.
pub fn lang_of(path: &Path) -> Option<&'static str> {
    Some(match path.extension()?.to_str()? {
        "rs" => "rust",
        "ts" | "tsx" | "mts" | "cts" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" | "pyi" => "python",
        "go" => "go",
        _ => return None,
    })
}

/// The servers crew knows without being told.
pub fn builtin() -> BTreeMap<String, Server> {
    let ts = Server::new("typescript-language-server", &["--stdio"]);
    BTreeMap::from([
        ("rust".into(), Server::new("rust-analyzer", &[])),
        ("typescript".into(), ts.clone()),
        ("javascript".into(), ts),
        (
            "python".into(),
            Server::new("pyright-langserver", &["--stdio"]),
        ),
        ("go".into(), Server::new("gopls", &[])),
    ])
}

#[derive(Debug, Default, Deserialize)]
struct File {
    #[serde(default)]
    servers: BTreeMap<String, Server>,
}

/// Parse one `lsp.json`; malformed content is an empty map, not a crash.
pub fn parse(text: &str) -> BTreeMap<String, Server> {
    serde_json::from_str::<File>(text)
        .map(|f| f.servers)
        .unwrap_or_default()
}

/// `overrides` on top of `base`: a language named in both takes the override.
pub fn merged(
    mut base: BTreeMap<String, Server>,
    overrides: BTreeMap<String, Server>,
) -> BTreeMap<String, Server> {
    base.extend(overrides);
    base
}

/// Where the override file lives.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("crew").join("lsp.json"))
}

/// The built-in table with the user's `lsp.json` merged over it.
pub fn load() -> BTreeMap<String, Server> {
    let user = config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| parse(&t))
        .unwrap_or_default();
    merged(builtin(), user)
}

/// `which` without a shell: the first `PATH` entry holding `cmd` as a file.
/// A command with a directory in it is checked where it says.
pub fn which(cmd: &str) -> Option<PathBuf> {
    let candidates = |p: &Path| -> Vec<PathBuf> {
        let mut v = vec![p.to_path_buf()];
        if cfg!(windows) {
            for ext in ["exe", "cmd", "bat"] {
                v.push(p.with_extension(ext));
            }
        }
        v
    };
    if cmd.contains(std::path::MAIN_SEPARATOR) || cmd.contains('/') {
        return candidates(Path::new(cmd)).into_iter().find(|p| p.is_file());
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| candidates(&dir.join(cmd)))
        .find(|p| p.is_file())
}

/// The binary that would serve `lang`, if it is both configured and on PATH.
pub fn available(lang: &str) -> Option<PathBuf> {
    which(&load().get(lang)?.command)
}

#[cfg(test)]
#[path = "servers_tests.rs"]
mod tests;
