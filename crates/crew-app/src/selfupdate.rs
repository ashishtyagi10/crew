//! In-place self-update for the **CLI** path (`crew --self-update`): download
//! the latest GitHub release for this platform and replace the running `crew`
//! binary, reporting progress on stdout. The in-app `/update` command instead
//! runs a background worker (see `update`/`updatefetch`) that shows progress in
//! the left-nav UPDATE card and auto-restarts; this standalone path stays as a
//! headless fallback you can run from any shell.
//!
//! Both paths go through the same [`crate::updatefetch::install`], and that
//! matters: this one used `self_update`'s own `update()`, whose Windows replace
//! step **removes the installation when it fails** (see `exereplace`). It was
//! being recommended as the workaround for the in-app update failing, which
//! would have made things worse rather than better.
use anyhow::Result;

/// Download and install the latest release over the running binary. Returns
/// once the binary is replaced (or was already current); the caller restarts
/// Crew to pick up the new version.
pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let (owner, name) = (
        crate::updatefetch::REPO_OWNER,
        crate::updatefetch::REPO_NAME,
    );
    println!("crew v{current} — checking github.com/{owner}/{name} for updates…\n");

    let latest = crate::updatefetch::latest_version()?;
    if !self_update::version::bump_is_greater(current, &latest).unwrap_or(false) {
        println!("✓ Already up to date (v{current}).");
        return Ok(());
    }

    println!("downloading v{latest}…");
    // `{e:#}` prints the whole context chain — which step failed and where —
    // rather than only the outermost message. On the CLI path there is no LOG
    // card to fall back on, so this is the only account the user gets.
    match crate::updatefetch::install(current) {
        Ok(v) => {
            println!("\n✓ Updated to {v}. Restart Crew to run it.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\n✗ Update failed: {e:#}");
            Err(e)
        }
    }
}
