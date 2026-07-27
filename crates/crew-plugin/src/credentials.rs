//! Provider credentials supplied from inside crew, rather than from the
//! environment. Stored as JSON next to `config.toml` — deliberately NOT inside
//! `CrewConfig`, which is user-visible, hand-edited and safe to paste around;
//! a key in there would leak the first time someone shared their config.
//!
//! Lives in `crew-plugin` because both consumers reach it here: the broker IS
//! this crate, and `crew-app` already depends on it.
//!
//! Not the macOS Keychain: crew ships Linux binaries too, so Keychain would
//! mean two code paths and a platform-specific failure mode for a v1. An
//! owner-only file is what `gh` and `aws` do. Keychain stays open as an upgrade.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The only variables this store will hold, so a key typed into the UI can
/// never name an arbitrary environment variable.
pub const VARS: [&str; 3] = [
    "DASHSCOPE_API_KEY",
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
];

/// The provider a variable authenticates, spelled as `CREW_PROVIDER` and
/// `pick_provider` spell it.
pub fn provider_for(var: &str) -> Option<&'static str> {
    match var {
        "DASHSCOPE_API_KEY" => Some("dashscope"),
        "OPENROUTER_API_KEY" => Some("openrouter"),
        "ANTHROPIC_API_KEY" => Some("anthropic"),
        _ => None,
    }
}

/// The on-disk shape. `provider` is the pin written when a key is saved: with
/// `pick_provider`'s fixed DashScope → OpenRouter → Anthropic order, supplying
/// an Anthropic key while a DashScope key exists would otherwise change
/// nothing the user can see.
#[derive(Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Store {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub keys: BTreeMap<String, String>,
}

/// Hand-written so the key VALUES can never be printed. A derived `Debug`
/// would put raw secrets into any `dbg!`, `assert_eq!` failure, `anyhow`
/// context or panic message that ever touches a `Store` — none of which the
/// author of that line would be thinking about. The names and the provider pin
/// are not secret and stay visible, so the output is still useful.
impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redacted: BTreeMap<&str, &str> = self
            .keys
            .keys()
            .map(|k| (k.as_str(), "<redacted>"))
            .collect();
        f.debug_struct("Store")
            .field("provider", &self.provider)
            .field("keys", &redacted)
            .finish()
    }
}

/// `<config_dir>/crew/credentials.json`, a sibling of `config.toml` — unless
/// `CREW_CREDENTIALS_PATH` is set (non-empty), in which case that path wins.
///
/// This exists purely for test isolation. The store is a process-global,
/// real-disk singleton with no equivalent of `CREW_PROJECT_DIR`'s CWD seam:
/// every other on-disk store here (`specialists`, `plugins`, `sessionlog`)
/// takes a base directory tests can point elsewhere, but credentials always
/// resolved through `dirs::config_dir()` with no override. That made
/// `broker::testenv::no_provider()` — which promises to force provider
/// discovery to fail even on a machine with real keys exported — silently
/// unable to keep that promise once a real `credentials.json` existed: reads
/// through `forced_provider()`/`shellenv::hydrate()` reached past the guard
/// straight to the real config directory. Do not remove this override to
/// "simplify" the function; it is what lets tests neutralise the store the
/// same way they neutralise the environment.
pub fn path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CREW_CREDENTIALS_PATH") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    dirs::config_dir().map(|d| d.join("crew").join("credentials.json"))
}

/// The stored credentials, or an empty store. EVERY failure — no config dir,
/// no file, unreadable, malformed JSON — reads as empty: a broken credentials
/// file must never stop crew from starting.
pub fn load() -> Store {
    path().map(|p| load_from(&p)).unwrap_or_default()
}

/// [`load`] from an explicit path (the testable half).
pub fn load_from(path: &Path) -> Store {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Store `value` for `var`, optionally moving the provider pin. An empty
/// `value` REMOVES the key rather than storing a blank — an empty
/// `ANTHROPIC_API_KEY` still outranks a valid OAuth profile, so a blank is
/// worse than nothing.
///
/// Never logs `value`.
pub fn save_key(var: &str, value: &str, provider: Option<&str>) -> anyhow::Result<()> {
    let path =
        path().ok_or_else(|| anyhow::anyhow!("no config directory to store credentials in"))?;
    save_key_at(&path, var, value, provider)
}

/// [`save_key`] at an explicit path (the testable half).
pub fn save_key_at(
    path: &Path,
    var: &str,
    value: &str,
    provider: Option<&str>,
) -> anyhow::Result<()> {
    if !VARS.contains(&var) {
        anyhow::bail!("{var} is not a provider key crew stores");
    }
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("credentials path has no parent directory"))?;
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let mut store = load_from(path);
    if value.is_empty() {
        store.keys.remove(var);
    } else {
        store.keys.insert(var.to_string(), value.to_string());
    }
    if let Some(p) = provider {
        store.provider = Some(p.to_string());
    }
    write_atomic(path, &serde_json::to_vec_pretty(&store)?)
}

/// Write via a same-directory temp file created 0600 BEFORE any content lands
/// in it, then rename over the target. There is never a moment when the
/// secret exists in a world-readable file, and a crash leaves either the old
/// file or the temp — never a truncated one.
///
/// Public because the credential store is not the only file that must never be
/// world-readable: `crew-app`'s `history` captures every line typed into the
/// input bar, which can include a secret typed into the wrong surface, and it
/// gets the same treatment through here rather than a second copy of this
/// reasoning. `std::fs::write` is the thing to avoid — it creates 0644 and can
/// only be chmod'd *after* the bytes have landed.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    // `<name>.tmp`, appended rather than substituted: `with_extension` would
    // have to know the target's extension (and this now writes an extensionless
    // history file too). For `credentials.json` both spell `credentials.json.tmp`.
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    // Remove any stale temp entry before creating our own. `create_new` alone
    // would fail forever after an interrupted write; removing first and then
    // refusing to open anything that still exists means we only ever write to
    // an inode we just made, at the mode we chose. `remove_file` unlinks a
    // symlink itself rather than following it, so a planted link is destroyed,
    // not written through.
    let _ = std::fs::remove_file(&tmp);
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
