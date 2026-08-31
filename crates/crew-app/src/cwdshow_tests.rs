use super::*;
// The display half's tests still reach the resolving half — they check what a
// resolved path LOOKS like, which needs both.
use crate::cwd::{canonical, resolve};

#[test]
fn fit_legend_keeps_tail_behind_ellipsis() {
    // fits → unchanged.
    assert_eq!(fit_legend("~/code/crew", 20), "~/code/crew");
    // too long → leading ellipsis + the last (max-1) chars (the deep tail).
    assert_eq!(fit_legend("~/a/b/c/deep", 6), "…/deep");
    // a zero budget is a no-op rather than a panic.
    assert_eq!(fit_legend("~/x", 0), "~/x");
}

/// `$HOME` here would make this a no-op on Windows — which is exactly how
/// the bug shipped: the legend showed `C:\Users\me\code` in full because
/// neither the home lookup nor the `/` separator applied there.
#[test]
fn display_abbreviates_home_on_this_platform() {
    let home = dirs::home_dir().expect("every supported platform has a home dir");
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(display(&home), "~");
    assert_eq!(display(&home.join("code")), format!("~{sep}code"));
    assert_eq!(
        display(&home.join("code").join("crew")),
        format!("~{sep}code{sep}crew")
    );
    // A path outside home is untouched.
    let outside = home
        .parent()
        .unwrap_or(&home)
        .join("definitely-not-home-xyz");
    assert_eq!(display(&outside), outside.to_string_lossy());
}

/// `std::fs::canonicalize` hands back `\\?\C:\Users\me` on Windows, and crew
/// canonicalizes both the saved start directory and every `cd` target — so
/// that prefix reached the legend, the pane spawn directory and the shell
/// prompt, and blocked the `~` abbreviation on top.
#[test]
fn the_windows_verbatim_prefix_is_unwrapped() {
    assert_eq!(strip_verbatim(r"\\?\C:\Users\me"), r"C:\Users\me");
    assert_eq!(strip_verbatim(r"\\?\D:\"), r"D:\");
    // Already plain, or POSIX — untouched.
    assert_eq!(strip_verbatim(r"C:\Users\me"), r"C:\Users\me");
    assert_eq!(strip_verbatim("/home/me"), "/home/me");
    // UNC keeps its prefix: it is load-bearing for network shares.
    assert_eq!(
        strip_verbatim(r"\\?\UNC\server\share"),
        r"\\?\UNC\server\share"
    );
    // Anything else after the prefix is left alone rather than guessed at.
    assert_eq!(strip_verbatim(r"\\?\Volume{abc}\x"), r"\\?\Volume{abc}\x");
}

/// The two fixes have to compose: unwrapping the prefix is what lets the
/// abbreviation recognise home in the first place.
#[test]
fn an_unwrapped_windows_path_then_abbreviates() {
    let raw = r"\\?\C:\Users\me\code\crew";
    assert_eq!(
        abbreviate(strip_verbatim(raw), r"C:\Users\me", '\\'),
        r"~\code\crew"
    );
}

/// The Windows half of the legend bug, proved from any machine: home came
/// from `$HOME` (unset on Windows) and the separator was a hardcoded `/`,
/// so `C:\Users\me\code` never abbreviated and the bar showed the whole
/// absolute path — the "long name" in the report.
#[test]
fn windows_paths_abbreviate_with_a_backslash() {
    assert_eq!(abbreviate(r"C:\Users\me", r"C:\Users\me", '\\'), "~");
    assert_eq!(
        abbreviate(r"C:\Users\me\code", r"C:\Users\me", '\\'),
        r"~\code"
    );
    assert_eq!(
        abbreviate(r"C:\Users\me\code\crew", r"C:\Users\me", '\\'),
        r"~\code\crew"
    );
    // A different user's directory is not "home"; prefix matching must not
    // fire on a partial component.
    assert_eq!(
        abbreviate(r"C:\Users\meredith\x", r"C:\Users\me", '\\'),
        r"C:\Users\meredith\x"
    );
    // A drive-root home keeps exactly one separator.
    assert_eq!(abbreviate(r"C:\code", r"C:\", '\\'), r"~\code");
}

#[test]
fn posix_paths_abbreviate_with_a_slash() {
    assert_eq!(abbreviate("/home/me", "/home/me", '/'), "~");
    assert_eq!(abbreviate("/home/me/code", "/home/me", '/'), "~/code");
    assert_eq!(
        abbreviate("/home/mere/x", "/home/me", '/'),
        "/home/mere/x",
        "a partial component match must not abbreviate"
    );
    assert_eq!(abbreviate("/etc", "/home/me", '/'), "/etc");
    // No home known → the path is returned untouched rather than mangled.
    assert_eq!(abbreviate("/etc", "", '/'), "/etc");
}

#[test]
fn cd_home_resolves_without_a_posix_home_variable() {
    let home = dirs::home_dir().expect("home dir");
    let canon = Some(canonical(&home));
    // `cd`, `cd ~` both mean home — via dirs, not $HOME.
    assert_eq!(resolve(Path::new(std::path::MAIN_SEPARATOR_STR), ""), canon);
    assert_eq!(
        resolve(Path::new(std::path::MAIN_SEPARATOR_STR), "~"),
        canon
    );
}
