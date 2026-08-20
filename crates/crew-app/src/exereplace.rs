//! Replacing the running binary, without the two things that broke it on
//! Windows.
//!
//! ## What went wrong
//!
//! `/update` used `self_replace`, whose Windows path is, in order:
//!
//! ```text
//! 1. rename the running crew.exe aside
//! 2. copy it into %TEMP% and SPAWN that copy, to delete the leftover
//! 3. move the newly downloaded exe into place
//! ```
//!
//! Step 2 executes a binary out of `%TEMP%`, which is precisely what a managed
//! machine's AppLocker/WDAC policy and most corporate antivirus refuse —
//! `Access denied`. And because step 2 fails *after* step 1, the function
//! returns an error having already moved `crew.exe` away, with nothing put
//! back. A failed update did not just fail: it **removed the installation**.
//!
//! ## What this does instead
//!
//! The same move `install.ps1` already performs successfully on those
//! machines, and nothing more:
//!
//! * **Rename aside, then rename in.** Windows locks a running image against
//!   being overwritten but permits renaming it — no second process, no
//!   `%TEMP%` execution, no administrator rights.
//! * **Roll back if the second rename fails.** The old binary goes back where
//!   it was, so a failure leaves a working crew rather than none.
//! * **Leave the aside file for [`sweep_leftovers`]** to remove on a later
//!   launch. It cannot be deleted now — it is the image this process is
//!   running from.
//!
//! Unix runs the same sequence even though POSIX would allow a single rename
//! over the open file. A `cfg` split would leave the Windows branch — the one
//! that broke, and the one carrying the rollback — untested by anything a
//! developer runs locally, and this session has already paid for that mistake
//! more than once. On Unix the aside file simply unlinks immediately.
//!
//! Both platforms require `new_exe` to sit **on the same volume** as `target`.
//! `fs::rename` cannot cross volumes on Windows, and the system temp directory
//! frequently is one — so callers stage the download beside the target, which
//! keeps it out of `%TEMP%` as a bonus.
use std::io;
use std::path::{Path, PathBuf};

/// Suffix marking a superseded binary awaiting cleanup. Distinctive enough to
/// sweep by prefix match without risking anything else in the directory.
const ASIDE_SUFFIX: &str = ".crew-old";

/// Where `target` is moved to while the new binary takes its place.
fn aside_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(ASIDE_SUFFIX);
    target.with_file_name(name)
}

/// Move `new_exe` into place at `target`, which may be the running binary.
///
/// On failure `target` is left as it was — see the module docs on why that
/// matters more than it sounds.
pub(crate) fn replace_running_exe(target: &Path, new_exe: &Path) -> io::Result<()> {
    // An extracted archive member may arrive without the executable bit.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(new_exe)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(new_exe, perms)?;
    }
    replace_with(target, new_exe, |from, to| std::fs::rename(from, to))
}

/// [`replace_running_exe`]'s sequence with the move injected.
///
/// The rollback only runs when the *second* move fails, and no arrangement of
/// real files reaches that on demand — an absent `new_exe` fails earlier, at
/// the metadata call. Testing it by hoping is how the first version of this
/// test passed with the rollback deleted; injecting the move is how it fails.
///
/// One algorithm on every platform, deliberately. Unix could replace the
/// directory entry with a single rename, but then the Windows path — the one
/// that broke, and the one carrying the rollback — would be code no test on a
/// developer's Mac ever executes.
fn replace_with<F>(target: &Path, new_exe: &Path, mut mv: F) -> io::Result<()>
where
    F: FnMut(&Path, &Path) -> io::Result<()>,
{
    let aside = aside_path(target);
    // A previous update may have left one behind; renaming onto it would fail.
    let _ = std::fs::remove_file(&aside);
    mv(target, &aside)?;
    match mv(new_exe, target) {
        Ok(()) => {
            // Best effort. On Windows this is the running image and will not
            // go until the process does, so `sweep_leftovers` gets it on a
            // later launch; on Unix it unlinks now.
            let _ = std::fs::remove_file(&aside);
            Ok(())
        }
        Err(e) => {
            // Put it back. Without this, a failure here is what leaves the
            // machine with no crew at all.
            mv(&aside, target)?;
            Err(e)
        }
    }
}

/// Delete superseded binaries left beside `exe` by an earlier update.
///
/// Best effort by design: the one from *this* process's own update is still
/// locked until it exits, so failures are expected and ignored. Returns how
/// many were removed, for the log line and the tests.
pub(crate) fn sweep_leftovers(exe: &Path) -> usize {
    let Some(dir) = exe.parent() else {
        return 0;
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(ASIDE_SUFFIX))
        .filter(|e| std::fs::remove_file(e.path()).is_ok())
        .count()
}

#[cfg(test)]
#[path = "exereplace_tests.rs"]
mod tests;
