//! Stored OAuth token sets, one per device-flow provider. Two backends, the
//! better one first: the OS keychain (`keychain`, the macOS `security` CLI,
//! probed never assumed) and an owner-only file (`credentials::write_atomic`,
//! 0600 before any byte lands) everywhere the keychain isn't.
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

/// The stored grant for `provider`: the keychain when one answers, else the
/// file (also checked when the keychain has no item — a grant stored before
/// a keychain existed, or under a store that fell back, stays readable).
pub(crate) fn load(provider: &str) -> Option<StoredToken> {
    load_via(
        super::keychain::bin().as_deref(),
        path().as_deref(),
        provider,
    )
}

/// The injected-backend half of [`load`].
pub(crate) fn load_via(
    bin: Option<&Path>,
    file: Option<&Path>,
    provider: &str,
) -> Option<StoredToken> {
    if let Some(bin) = bin {
        if let Some(secret) = super::keychain::load_with(bin, provider) {
            if let Ok(t) = serde_json::from_str::<StoredToken>(&secret) {
                return Some(t);
            }
        }
    }
    load_at(file?, provider)
}

pub(crate) fn load_at(path: &Path, provider: &str) -> Option<StoredToken> {
    load_all(path).remove(provider)
}

/// Store `provider`'s grant: the keychain when one answers, else the 0600
/// file (atomic — see `credentials::write_atomic`). A keychain that refuses
/// costs nothing but the better backend.
pub(crate) fn store(provider: &str, tok: StoredToken) -> anyhow::Result<()> {
    store_via(
        super::keychain::bin().as_deref(),
        path().as_deref(),
        provider,
        tok,
    )
}

/// The injected-backend half of [`store`].
pub(crate) fn store_via(
    bin: Option<&Path>,
    file: Option<&Path>,
    provider: &str,
    tok: StoredToken,
) -> anyhow::Result<()> {
    if let Some(bin) = bin {
        if let Ok(json) = serde_json::to_string(&tok) {
            if super::keychain::store_with(bin, provider, &json) {
                return Ok(());
            }
        }
    }
    let file = file.ok_or_else(|| anyhow::anyhow!("no config directory to store tokens in"))?;
    store_at(file, provider, tok)
}

/// What `/doctor` says about where grants live.
pub(crate) fn backend_note() -> &'static str {
    if super::keychain::bin().is_some() {
        "OS keychain (macOS `security`)"
    } else {
        "0600 file (no keychain on this system)"
    }
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

/// Drop `provider`'s grant (a hard refresh failure discards the dead set) —
/// from BOTH backends, so a dead grant cannot resurface from the one a
/// store call happened not to use.
pub(crate) fn clear(provider: &str) {
    if let Some(bin) = super::keychain::bin() {
        super::keychain::delete_with(&bin, provider);
    }
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
