//! The release-archive download, with a budget a slow link can actually meet.
//!
//! This replaces `self_update::Download`, which builds its client with a bare
//! `reqwest::blocking::ClientBuilder::new()` — and reqwest's *blocking* client
//! defaults to a 30-second timeout covering the whole request, the body read
//! included (the async client has no such default). The macOS archive is about
//! 11 MB, so `/update` had to sustain ~375 KB/s or fail, and it failed the same
//! way on every retry, silently taking the 6-hourly background check with it:
//!
//! ```text
//! update failed: ReqwestError: error decoding response body: operation timed out
//! ```
//!
//! A self-updater must give up on a connection that is *dead*, never on one
//! that is merely slow. So nothing here bounds the transfer as a whole; the
//! timeouts bound how long we wait for the connection, and then for the next
//! bytes to arrive. A download that keeps making progress keeps going, however
//! long it takes.
use std::io::Write;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::header::{ACCEPT, USER_AGENT};

/// How long to wait for the connection itself.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// How long to wait for the *next* bytes of the body. Generous: a stalled
/// download still ends in an error rather than hanging the update worker
/// forever, but a link that trickles is never mistaken for a broken one.
const STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Stream the asset at `url` into `dest`, returning the bytes written.
///
/// `ua` is the User-Agent GitHub requires; `Accept: application/octet-stream`
/// is what turns a release-asset *API* URL into the asset instead of its JSON
/// metadata.
pub(crate) fn download_to(url: &str, ua: &str, dest: &mut impl Write) -> Result<u64> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("starting the download runtime")?;
    rt.block_on(stream(url, ua, dest))
}

async fn stream(url: &str, ua: &str, dest: &mut impl Write) -> Result<u64> {
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(STALL_TIMEOUT)
        .build()
        .context("building the download client")?;
    let mut resp = client
        .get(url)
        .header(ACCEPT, "application/octet-stream")
        .header(USER_AGENT, ua)
        .send()
        .await
        .context("sending the download request")?;
    let status = resp.status();
    if !status.is_success() {
        bail!("the download request failed with status {status}");
    }
    let mut written = 0u64;
    while let Some(chunk) = resp.chunk().await.context("reading the download body")? {
        dest.write_all(&chunk)?;
        written += chunk.len() as u64;
    }
    dest.flush()?;
    if written == 0 {
        bail!("the download was empty");
    }
    Ok(written)
}

#[cfg(test)]
#[path = "updatedl_tests.rs"]
mod tests;
