//! The inbox's Google grant: the user's own OAuth client and the refresh
//! token it was given, in `gmail.json` beside `credentials.json`, written
//! 0600 the way that file is (`credentials::write_atomic`).
//!
//! The client is the user's, made in their own Google Cloud project. Gmail's
//! read scopes are restricted, so crew cannot ship a client of its own
//! (goal 2026-10-20-inbox). Its "secret" is not one: Google says a desktop
//! client's secret cannot be kept, and asks for PKCE instead. The refresh
//! token is the real credential, and nothing here prints it.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A desktop OAuth client, as Google's console downloads it.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Client {
    pub id: String,
    pub secret: String,
}

/// What a sign-in leaves: the client it used and the refresh token.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Grant {
    pub client: Client,
    pub refresh_token: String,
    /// The account, once the profile has been read; shown in the legend.
    #[serde(default)]
    pub email: String,
}

/// Redacted: the refresh token and the secret never reach a log or a panic.
impl std::fmt::Debug for Grant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Grant({}, {:?}, <redacted>)", self.client.id, self.email)
    }
}

/// The client in the JSON the console's download button gives
/// (`{"installed": {"client_id": …, "client_secret": …}}`), or one pasted as
/// the bare `{"client_id": …, "client_secret": …}`.
pub(crate) fn client_from_json(text: &str) -> Option<Client> {
    let v: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
    let c = v.get("installed").unwrap_or(&v);
    let field = |k: &str| c[k].as_str().map(str::trim).filter(|s| !s.is_empty());
    Some(Client {
        id: field("client_id")?.to_string(),
        secret: field("client_secret")?.to_string(),
    })
}

/// The newest `client_secret_*.json` in `dir` — where a browser saves the
/// console's download — so setup ends at "Download JSON".
pub(crate) fn downloaded_client(dir: &Path) -> Option<(PathBuf, Client)> {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("client_secret_") && name.ends_with(".json")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    found.sort();
    let (_, path) = found.pop()?;
    let client = client_from_json(&std::fs::read_to_string(&path).ok()?)?;
    Some((path, client))
}

/// `gmail.json`, beside `credentials.json` (and moved with it by
/// `CREW_CREDENTIALS_PATH`).
pub(crate) fn path() -> Option<PathBuf> {
    // A test never reads or deletes the user's real grant: `credentials` is
    // built without cfg(test) here, so its own guard does not reach us.
    if cfg!(test) {
        return None;
    }
    Some(crew_plugin::credentials::path()?.with_file_name("gmail.json"))
}

pub(crate) fn load_from(path: &Path) -> Option<Grant> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub(crate) fn save_to(path: &Path, grant: &Grant) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(grant)?;
    crew_plugin::credentials::write_atomic(path, text.as_bytes())
}

pub(crate) fn load() -> Option<Grant> {
    load_from(&path()?)
}

pub(crate) fn save(grant: &Grant) -> anyhow::Result<()> {
    let p = path().ok_or_else(|| anyhow::anyhow!("no config directory"))?;
    save_to(&p, grant)
}

/// Sign out: the file goes. (Revoking at Google is `signin::revoke`.)
pub(crate) fn clear() {
    if let Some(p) = path() {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
#[path = "grant_tests.rs"]
mod tests;
