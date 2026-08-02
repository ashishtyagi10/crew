//! OAuth grants in the OS keychain, via the macOS `security` CLI as a
//! subprocess — probed, never assumed, so a machine without it (Linux, a
//! stripped container) falls cleanly back to the 0600 file (`tokens`
//! dispatches; `/doctor` states which backend holds the grants).
//!
//! One generic-password item per provider: service [`SERVICE`], account =
//! the registry provider name. The secret is the serialized grant.
//!
//! NEVER log the secret, and never put command output into an error: on any
//! failure these functions answer `false`/`None` and the caller falls back —
//! a keychain that misbehaves must cost nothing but the better backend.
//! (The secret does ride `security`'s argv for the moment of the call, as
//! every `security`-scripting tool accepts; the alternative — an interactive
//! `-w` prompt — cannot be driven from a broker.)
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::probe::run_split;

/// The keychain service every crew grant lives under.
pub(crate) const SERVICE: &str = "crew-oauth";

/// `security` is local and fast; anything slower is a hang worth killing.
const SECURITY_TIMEOUT: Duration = Duration::from_secs(5);

/// The `security` binary to drive, or `None` for "no keychain here".
/// `CREW_SECURITY_BIN` overrides (the test seam: point it at a fake, or set
/// it empty to force the file backend); otherwise the stock macOS path, only
/// when it actually exists.
pub(crate) fn bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CREW_SECURITY_BIN") {
        return (!p.is_empty()).then(|| PathBuf::from(p));
    }
    let stock = Path::new("/usr/bin/security");
    stock.exists().then(|| stock.to_path_buf())
}

/// Store `secret` for `provider` (`-U` updates in place). `false` = the
/// keychain refused; the caller falls back to the file.
pub(crate) fn store_with(bin: &Path, provider: &str, secret: &str) -> bool {
    let args = [
        "add-generic-password",
        "-U",
        "-s",
        SERVICE,
        "-a",
        provider,
        "-w",
        secret,
    ];
    matches!(
        run_split(&bin.to_string_lossy(), &args, SECURITY_TIMEOUT),
        Some((true, ..))
    )
}

/// The stored secret for `provider`, if the keychain holds one. Reads
/// STDOUT only — `security` prints the password there and its warnings to
/// stderr, and a merged stream would corrupt the secret.
pub(crate) fn load_with(bin: &Path, provider: &str) -> Option<String> {
    let args = ["find-generic-password", "-s", SERVICE, "-a", provider, "-w"];
    match run_split(&bin.to_string_lossy(), &args, SECURITY_TIMEOUT)? {
        (true, out, _) => {
            let secret = out.trim_end_matches('\n').to_string();
            (!secret.is_empty()).then_some(secret)
        }
        _ => None,
    }
}

/// Drop `provider`'s item, best-effort (absent already counts as dropped).
pub(crate) fn delete_with(bin: &Path, provider: &str) {
    let args = ["delete-generic-password", "-s", SERVICE, "-a", provider];
    let _ = run_split(&bin.to_string_lossy(), &args, SECURITY_TIMEOUT);
}

#[cfg(test)]
#[path = "keychain_tests.rs"]
mod tests;
