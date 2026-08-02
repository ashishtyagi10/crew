//! Stored OAuth token sets, one per device-flow provider. This iteration's
//! backend is an owner-only file (`credentials::write_atomic`, 0600 before
//! any byte lands); the keychain backend layers in front in the next commit.
//!
//! States and timestamps are printable; TOKEN VALUES NEVER ARE — `Debug` is
//! hand-written for the same reason `credentials::Store`'s is, and nothing
//! here ever formats a token into an error or log line.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Refresh this many seconds BEFORE the server's expiry, so a token is never
/// presented in its final moments and rejected mid-flight.
pub(crate) const EXPIRY_SKEW_SECS: u64 = 30;

/// One provider's stored grant.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct StoredToken {
    pub access: String,
    pub refresh: Option<String>,
    /// Unix seconds after which `access` must be refreshed (skew applied at
    /// store time). `u64::MAX` when the server declared no expiry.
    pub expires_at: u64,
    /// The API host this token is valid against, when the server said
    /// (Qwen's differs from its key-shaped endpoint).
    pub resource: Option<String>,
}

impl std::fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredToken")
            .field("access", &"<redacted>")
            .field("refresh", &self.refresh.as_ref().map(|_| "<redacted>"))
            .field("expires_at", &self.expires_at)
            .field("resource", &self.resource)
            .finish()
    }
}

/// A granted [`crew_hive::deviceflow::TokenSet`] as it is stored: expiry
/// resolved against `now`, skew applied. Pure, so the arithmetic is testable
/// without a clock.
pub(crate) fn stored_from(t: &crew_hive::deviceflow::TokenSet, now: u64) -> StoredToken {
    StoredToken {
        access: t.access_token.clone(),
        refresh: t.refresh_token.clone(),
        expires_at: t
            .expires_in
            .map_or(u64::MAX, |s| now + s.saturating_sub(EXPIRY_SKEW_SECS)),
        resource: t.resource_url.clone(),
    }
}

/// Whether `t` still serves at `now` (the skew was applied at store time).
pub(crate) fn is_fresh(t: &StoredToken, now: u64) -> bool {
    now < t.expires_at
}

pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `tokens.json`, a sibling of `credentials.json` — resolved THROUGH the
/// credential store's own path (so its `CREW_CREDENTIALS_PATH` test seam
/// redirects both stores at once: a test that isolates one on a signed-in
/// machine must never read, or clobber, the user's real grants either).
pub(crate) fn path() -> Option<PathBuf> {
    let creds = crate::credentials::path()?;
    Some(creds.parent()?.join("tokens.json"))
}

/// The whole on-disk map. Every failure reads as empty — a broken token file
/// must never stop crew from starting (the flow simply re-runs).
fn load_all(path: &Path) -> BTreeMap<String, StoredToken> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// The stored grant for `provider`, if any.
pub(crate) fn load(provider: &str) -> Option<StoredToken> {
    load_at(&path()?, provider)
}

pub(crate) fn load_at(path: &Path, provider: &str) -> Option<StoredToken> {
    load_all(path).remove(provider)
}

/// Store `provider`'s grant (0600, atomic — see `credentials::write_atomic`).
pub(crate) fn store(provider: &str, tok: StoredToken) -> anyhow::Result<()> {
    let path = path().ok_or_else(|| anyhow::anyhow!("no config directory to store tokens in"))?;
    store_at(&path, provider, tok)
}

pub(crate) fn store_at(path: &Path, provider: &str, tok: StoredToken) -> anyhow::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("token path has no parent directory"))?;
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let mut all = load_all(path);
    all.insert(provider.to_string(), tok);
    crate::credentials::write_atomic(path, &serde_json::to_vec_pretty(&all)?)
}

/// Drop `provider`'s grant (a hard refresh failure discards the dead set).
pub(crate) fn clear(provider: &str) {
    let Some(path) = path() else { return };
    clear_at(&path, provider);
}

pub(crate) fn clear_at(path: &Path, provider: &str) {
    let mut all = load_all(path);
    if all.remove(provider).is_some() {
        if let Ok(bytes) = serde_json::to_vec_pretty(&all) {
            let _ = crate::credentials::write_atomic(path, &bytes);
        }
    }
}

#[cfg(test)]
#[path = "tokens_tests.rs"]
mod tests;
