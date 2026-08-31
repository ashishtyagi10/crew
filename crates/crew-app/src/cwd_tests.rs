use super::*;

#[test]
fn cd_arg_parses() {
    assert_eq!(cd_arg("cd"), Some(""));
    assert_eq!(cd_arg("cd /tmp"), Some("/tmp"));
    assert_eq!(cd_arg("  cd   foo/bar "), Some("foo/bar"));
    assert_eq!(cd_arg("cdx"), None);
    assert_eq!(cd_arg("ls"), None);
}

#[test]
fn resolve_relative_and_absolute() {
    let base = canonical(&std::env::temp_dir());
    // "." resolves back to base.
    assert_eq!(resolve(&base, "."), Some(base.clone()));
    // an absolute existing dir is kept.
    assert_eq!(resolve(&base, base.to_str().unwrap()), Some(base.clone()));
    // a non-existent path resolves to None.
    assert_eq!(resolve(&base, "definitely-not-here-xyz"), None);
}

#[test]
fn resolve_expands_env_var() {
    let base = canonical(&std::env::temp_dir());
    std::env::set_var("CREW_RESOLVE_DIR", base.to_str().unwrap());
    // `$VAR` expands to an absolute existing dir.
    assert_eq!(resolve(Path::new("/"), "$CREW_RESOLVE_DIR"), Some(base));
}

#[test]
fn resolved_start_prefers_valid_saved_dir() {
    let base = canonical(&std::env::temp_dir());
    // a saved dir that exists is used
    assert_eq!(resolved_start(Some(base.to_str().unwrap())), base);
    // a missing saved dir, or none, falls back to the process cwd
    let fallback = initial();
    assert_eq!(resolved_start(Some("/no/such/dir/xyz")), fallback);
    assert_eq!(resolved_start(None), fallback);
}

/// The reported bug: launched from Explorer, Windows sets the CWD to the
/// folder holding `crew.exe`, so every pane opened inside the unzipped
/// release directory and the prompt read
/// `PS C:\Users\me\Downloads\crew-v0.17.10-x86_64-pc-windows-msvc>`.
#[test]
fn the_directory_holding_our_own_exe_is_not_a_place_to_work() {
    let exe_dir = std::env::current_exe()
        .expect("current_exe")
        .parent()
        .expect("exe has a parent")
        .to_path_buf();
    assert!(
        is_launcher_cwd(&exe_dir),
        "{exe_dir:?} holds the running binary — a launcher put us here, \
         not a person"
    );
    assert_eq!(
        initial_from(Some(exe_dir.clone())),
        dirs::home_dir().expect("home dir"),
        "started in the exe's own folder — panes would open inside the \
         unzipped release download instead of somewhere useful"
    );
    // …while a directory a person picked is passed straight through.
    let chosen = std::env::temp_dir();
    assert_eq!(initial_from(Some(chosen.clone())), chosen);
    // …and no CWD at all still lands somewhere real.
    assert_eq!(initial_from(None), dirs::home_dir().expect("home dir"));
}

/// …and the root, which is what launchd hands a macOS Dock launch.
#[test]
fn a_filesystem_root_is_not_a_place_to_work() {
    assert!(is_launcher_cwd(Path::new(std::path::MAIN_SEPARATOR_STR)));
    if cfg!(windows) {
        assert!(is_launcher_cwd(Path::new("C:\\")));
    }
}

/// But a directory the user actually chose is kept — including the release
/// folder itself, if they `cd`'d there and ran it by hand.
#[test]
fn a_directory_a_person_chose_is_kept() {
    let tmp = std::env::temp_dir();
    assert!(
        !is_launcher_cwd(&tmp),
        "a normal directory must be left alone, or every launch would \
         ignore where it was started from"
    );
}
