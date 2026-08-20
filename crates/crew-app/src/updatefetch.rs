//! The GitHub side of the self-update: the worker thread that streams stage
//! messages, and the calls it makes — query the latest release version, and
//! download+install it over the running binary. Kept apart from `update`'s
//! UI/state so the network work stays self-contained on the worker thread.
use std::sync::mpsc::{channel, Receiver, Sender};

use anyhow::{anyhow, Context, Result};
use self_update::backends::github::ReleaseList;

use crate::applog::LogLevel;
use crate::update::UpdateMsg;

pub(crate) const REPO_OWNER: &str = "ashishtyagi10";
pub(crate) const REPO_NAME: &str = "crew";

/// Spawn the background update worker; the returned receiver streams its stages.
///
/// `log` also receives the outcome. The stage messages only reach a nav card
/// that clears after a few seconds, so a failed update used to leave **no
/// durable trace at all** — the whole diagnosis of the Windows `Access denied`
/// started from a user who could only report that it "failed". The LOG line
/// persists to `activity.log`, and carries the full error chain rather than
/// the card's one-line summary.
pub(crate) fn spawn_worker(log: Sender<(LogLevel, String)>) -> Receiver<UpdateMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || run_update(&tx, &log));
    rx
}

/// Worker body: check GitHub, and download+install when a newer release exists.
fn run_update(tx: &Sender<UpdateMsg>, log: &Sender<(LogLevel, String)>) {
    let current = env!("CARGO_PKG_VERSION");
    let _ = tx.send(UpdateMsg::Checking);
    match latest_version() {
        Ok(latest) => {
            let newer = self_update::version::bump_is_greater(current, &latest).unwrap_or(false);
            if !newer {
                let _ = tx.send(UpdateMsg::UpToDate(current.to_string()));
                return;
            }
            let _ = tx.send(UpdateMsg::Downloading(latest));
            match install(current) {
                Ok(v) => {
                    let _ = log.send((LogLevel::Info, format!("update: installed v{v}")));
                    let _ = tx.send(UpdateMsg::Installed(v));
                }
                Err(e) => {
                    // `{e:#}` gives the whole `with_context` chain — which step
                    // failed and on which path — not just the outermost line.
                    let _ = log.send((LogLevel::Error, format!("update failed: {e:#}")));
                    let _ = tx.send(UpdateMsg::Failed(short_err(&e)));
                }
            }
        }
        Err(e) => {
            let _ = log.send((LogLevel::Error, format!("update check failed: {e:#}")));
            let _ = tx.send(UpdateMsg::Failed(short_err(&e)));
        }
    }
}

/// A one-line, card-sized error string (first line, detail trimmed off).
fn short_err(e: &anyhow::Error) -> String {
    e.to_string()
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// The newest release tag on GitHub (e.g. "0.6.0"), without the `v` prefix.
pub(crate) fn latest_version() -> Result<String> {
    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;
    let latest = releases
        .first()
        .ok_or_else(|| anyhow!("no releases found"))?;
    Ok(latest.version.clone())
}

/// Download the latest release for this platform and replace the running
/// binary. Returns the version now on disk.
///
/// Assembled from `self_update`'s pieces rather than calling its `update()`,
/// because that ends in `self_replace`, whose Windows path **destroys the
/// installation when it fails**: it renames the running `crew.exe` aside, then
/// copies itself into `%TEMP%` and spawns that copy to clean up — which a
/// managed machine's AppLocker/antivirus refuses with `Access denied` — and
/// returns the error with nothing put back at the original path.
///
/// So `self_update` still does what it is good at (finding the release,
/// matching the asset for this target, unpacking the archive) and
/// [`crate::exereplace`] does the move, the way `install.ps1` already does it
/// on those same machines.
///
/// The archive is staged **beside the target binary**, not in the system temp
/// directory. Two reasons: `fs::rename` cannot cross volumes on Windows and
/// `%TEMP%` frequently is one, and staging out of `%TEMP%` avoids the very
/// directory corporate policy is most likely to block.
pub(crate) fn install(current: &str) -> Result<String> {
    let exe = std::env::current_exe().context("locating the running binary")?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow!("the running binary has no parent directory"))?;

    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;
    let release = releases
        .first()
        .ok_or_else(|| anyhow!("no releases found"))?;
    let target = self_update::get_target();
    let asset = release
        .asset_for(target, None)
        .ok_or_else(|| anyhow!("release {} has no asset for {target}", release.version))?;

    // Staged next to the binary we are replacing — see the note above.
    let staging = tempfile::Builder::new()
        .prefix(".crew-update-")
        .tempdir_in(dir)
        .with_context(|| format!("creating a staging directory in {}", dir.display()))?;
    let archive_path = staging.path().join(&asset.name);
    let mut archive = std::fs::File::create(&archive_path)?;

    let mut download = self_update::Download::from_url(&asset.download_url);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        format!("crew/{current}").parse().unwrap(),
    );
    download.set_headers(headers);
    download.show_progress(false);
    download
        .download_to(&mut archive)
        .with_context(|| format!("downloading {}", asset.name))?;
    drop(archive);

    // `crew` on Unix, `crew.exe` on Windows — the name inside the archive.
    let bin_name = format!("crew{}", std::env::consts::EXE_SUFFIX);
    self_update::Extract::from_source(&archive_path)
        .extract_file(staging.path(), &bin_name)
        .with_context(|| format!("extracting {bin_name} from {}", asset.name))?;

    crate::exereplace::replace_running_exe(&exe, &staging.path().join(&bin_name))
        .with_context(|| format!("replacing {}", exe.display()))?;
    Ok(release.version.clone())
}
