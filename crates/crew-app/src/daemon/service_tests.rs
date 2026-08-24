//! What the unit file says decides whether the resident actually comes back at login, and a
//! wrong one fails invisibly — installed, listed, never running. So the contents are pinned.
use super::*;

fn exe() -> PathBuf {
    PathBuf::from("/Users/x/.local/bin/crew")
}

/// A launchd agent starts with a minimal environment and no login PATH. A bare `crew` would work
/// when tested from a terminal and silently never start at boot.
#[test]
fn the_macos_agent_runs_an_absolute_path_with_the_daemon_subcommand() {
    let u = macos_unit(&exe());
    assert!(
        u.body.contains("<string>/Users/x/.local/bin/crew</string>"),
        "the program is the absolute exe path:\n{}",
        u.body
    );
    assert!(u.body.contains("<string>daemon</string>"));
    assert!(u.body.contains("<string>run</string>"));
    assert!(
        u.body.contains("<key>RunAtLoad</key>"),
        "it starts at login"
    );
}

/// Install writes a path derived from LABEL and uninstall looks one up the same way. If they
/// ever disagree, uninstall reports success while leaving the agent in place.
#[test]
fn install_and_uninstall_name_the_same_file() {
    let u = macos_unit(&exe());
    assert!(u.rel_path.contains(LABEL));
    assert!(u.activate.iter().any(|a| a == &u.rel_path));
    assert!(u.deactivate.iter().any(|a| a == &u.rel_path));
    let l = linux_unit(&exe());
    assert!(l.rel_path.contains(LABEL));
    assert!(l.deactivate.iter().any(|a| a.contains(LABEL)));
}

#[test]
fn the_linux_unit_execs_the_daemon_and_installs_into_the_user_session() {
    let u = linux_unit(&exe());
    assert!(u
        .body
        .contains("ExecStart=/Users/x/.local/bin/crew daemon run"));
    assert!(u.body.contains("WantedBy=default.target"));
    assert!(u.rel_path.starts_with(".config/systemd/user/"));
}

/// Everything stays inside $HOME: no sudo, no system-wide daemon, nothing a user cannot undo.
#[test]
fn every_unit_path_is_relative_to_home() {
    for u in [macos_unit(&exe()), linux_unit(&exe())] {
        assert!(!u.rel_path.starts_with('/'), "{} escapes $HOME", u.rel_path);
        assert!(!u.rel_path.contains(".."), "{} climbs out", u.rel_path);
    }
}

fn tmp_home(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crewd-home-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn writing_creates_the_file_and_removing_takes_it_away() {
    let home = tmp_home("rt");
    let u = macos_unit(&exe());
    assert!(!is_installed(&home, &u), "nothing installed to begin with");
    let path = write_unit(&home, &u).expect("write");
    assert!(path.exists());
    assert!(is_installed(&home, &u));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), u.body);
    assert!(
        remove_unit(&home, &u).unwrap(),
        "it reports having removed one"
    );
    assert!(!is_installed(&home, &u));
    let _ = std::fs::remove_dir_all(&home);
}

/// Uninstalling something that was never installed is a plain "nothing to do", not an error the
/// user has to interpret.
#[test]
fn removing_what_was_never_installed_is_not_an_error() {
    let home = tmp_home("absent");
    let u = macos_unit(&exe());
    assert!(!remove_unit(&home, &u).unwrap(), "nothing to remove");
}

/// A second install over an existing agent must land the current binary's path, not leave the
/// old one — otherwise moving or reinstalling crew leaves a service pointing at a dead file.
#[test]
fn installing_twice_rewrites_the_program_path() {
    let home = tmp_home("rewrite");
    write_unit(&home, &macos_unit(&PathBuf::from("/old/crew"))).unwrap();
    let u = macos_unit(&PathBuf::from("/new/crew"));
    let path = write_unit(&home, &u).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("/new/crew"), "the new path is written");
    assert!(!body.contains("/old/crew"), "the old path is gone");
    let _ = std::fs::remove_dir_all(&home);
}

/// A failing activation must be reported, not swallowed — an install that says nothing while
/// launchctl refused is exactly the "installed but never runs" state this module exists to avoid.
#[test]
fn a_failing_activation_step_reports_the_error() {
    let home = std::env::temp_dir();
    let err = run_step(&home, &["/nonexistent/launchctl".to_string()]).unwrap_err();
    assert!(!err.is_empty(), "the failure carries a message");
}

/// The consent guard, as a source-tree scan.
///
/// The whole safety story of this module is "the user typed `crew daemon install`". A release,
/// an auto-update, a first-run wizard, or a well-meaning night-loop iteration that calls the
/// installer for them turns a bad build into a login loop instead of an `/update`. So nothing
/// outside the CLI subcommand may reach the installer at all.
#[test]
fn nothing_installs_the_service_without_being_asked() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(
        files.len() > 50,
        "the walk found only {} files",
        files.len()
    );

    let mut offenders = Vec::new();
    for f in &files {
        let name = f
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // The installer itself, its tests, and the one CLI subcommand that exists to call it.
        if name.starts_with("service") || name.starts_with("cli") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains("write_unit(") || line.contains("unit_for_host(") {
                offenders.push(format!("{name}:{}", n + 1));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these call the service installer outside `crew daemon install`, which would put a \
         background service on a machine whose owner never asked for one: {offenders:?}"
    );
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}
