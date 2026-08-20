//! The case that matters is the *failing* one. `self_replace` got the happy
//! path right and lost the installation on the sad path, so these tests spend
//! most of their effort on what happens when the second move fails.
use super::*;
use std::fs;
use std::io;

/// A directory with a stand-in "binary" in it.
fn staged(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, body).expect("write");
    p
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crew-exereplace-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("mkdir");
    d
}

#[test]
fn the_new_binary_takes_the_target_s_place() {
    let d = tmpdir("happy");
    let target = staged(&d, "crew", "OLD");
    let fresh = staged(&d, "crew.new", "NEW");

    replace_running_exe(&target, &fresh).expect("replace");

    assert_eq!(fs::read_to_string(&target).unwrap(), "NEW");
    assert!(!fresh.exists(), "the staged file was moved, not copied");
    let _ = fs::remove_dir_all(&d);
}

/// The `self_replace` bug, as a test: when the move of the *new* binary fails,
/// the old one must be put back. That crate renamed the running exe aside,
/// failed on the next step, and returned an error leaving nothing at the
/// original path — so a failed update uninstalled crew.
///
/// The move is injected because no arrangement of real files makes the second
/// rename fail on demand. Written with real files, this test passed with the
/// rollback deleted.
#[test]
fn a_failure_of_the_second_move_puts_the_original_back() {
    let d = tmpdir("rollback");
    let target = staged(&d, "crew", "OLD");
    let fresh = staged(&d, "crew.new", "NEW");

    let fresh_for_closure = fresh.clone();
    let err = replace_with(&target, &fresh, |from, to| {
        if from == fresh_for_closure {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "access denied",
            ));
        }
        fs::rename(from, to)
    })
    .expect_err("the injected move must fail");
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

    assert!(
        target.exists(),
        "the original binary is gone after a failed update — the installation \
         would be destroyed, which is exactly the bug this replaces"
    );
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        "OLD",
        "the original binary must be exactly what it was before the attempt"
    );
    assert!(
        !aside_path(&target).exists(),
        "the aside copy was left behind — the rollback moved it, it did not copy it"
    );
    let _ = fs::remove_dir_all(&d);
}

/// The earlier, cheaper failure: nothing was downloaded at all. It must also
/// leave the binary alone — it just fails before the sequence starts.
#[test]
fn a_missing_download_never_touches_the_target() {
    let d = tmpdir("nodownload");
    let target = staged(&d, "crew", "OLD");

    replace_running_exe(&target, &d.join("not-downloaded"))
        .expect_err("replacing from a missing file must fail");

    assert_eq!(fs::read_to_string(&target).unwrap(), "OLD");
    assert!(
        !aside_path(&target).exists(),
        "the target was moved aside before the download was even checked"
    );
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn a_leftover_from_an_earlier_update_does_not_block_the_next_one() {
    let d = tmpdir("stale");
    let target = staged(&d, "crew", "OLD");
    // Simulate the file a previous in-place update could not delete because it
    // was still the running image.
    staged(&d, &format!("crew{ASIDE_SUFFIX}"), "OLDER");
    let fresh = staged(&d, "crew.new", "NEW");

    replace_running_exe(&target, &fresh).expect("a stale aside must not block");
    assert_eq!(fs::read_to_string(&target).unwrap(), "NEW");
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn the_sweep_removes_superseded_binaries_and_nothing_else() {
    let d = tmpdir("sweep");
    let exe = staged(&d, "crew", "RUNNING");
    staged(&d, &format!("crew{ASIDE_SUFFIX}"), "OLD");
    staged(&d, &format!("crew-2{ASIDE_SUFFIX}"), "OLDER");
    staged(&d, "config.toml", "keep me");

    assert_eq!(sweep_leftovers(&exe), 2, "both superseded binaries go");
    assert!(exe.exists(), "the running binary must never be swept");
    assert!(
        d.join("config.toml").exists(),
        "unrelated files are untouched"
    );
    assert_eq!(sweep_leftovers(&exe), 0, "a second sweep finds nothing");
    let _ = fs::remove_dir_all(&d);
}

/// The sweep runs at startup on every launch; it must be silent about a
/// directory it cannot read rather than failing the launch.
#[test]
fn the_sweep_is_quiet_when_there_is_nothing_to_do() {
    assert_eq!(sweep_leftovers(Path::new("/definitely/not/here/crew")), 0);
    assert_eq!(sweep_leftovers(Path::new("/")), 0);
}
