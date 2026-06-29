//! In-place self-update: download the latest GitHub release for this platform
//! and replace the running `crew` binary, printing a progress bar. Invoked as a
//! subprocess (`crew --self-update`) by the `/update` command so its progress
//! renders in a normal terminal pane — no shell, no GUI re-init.
use anyhow::Result;
use self_update::backends::github::Update;

const REPO_OWNER: &str = "ashishtyagi10";
const REPO_NAME: &str = "crew";

/// Download and install the latest release over the running binary, streaming a
/// progress bar to stdout. Returns once the binary is replaced (or already
/// current); the caller restarts Crew to pick up the new version.
pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("crew v{current} — checking github.com/{REPO_OWNER}/{REPO_NAME} for updates…\n");
    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("crew")
        .current_version(current)
        .show_download_progress(true)
        .no_confirm(true)
        .build()?
        .update()?;
    if status.updated() {
        println!(
            "\n✓ Updated to {}. Restart Crew (close this pane and relaunch) to run it.",
            status.version()
        );
    } else {
        println!("\n✓ Already up to date (v{current}).");
    }
    Ok(())
}
