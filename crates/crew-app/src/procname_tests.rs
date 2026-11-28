use super::*;

#[test]
fn names_this_test_process() {
    // Resolve our own PID — proves the refresh + lookup path works end to end
    // without depending on any particular external program being alive.
    let me = std::process::id();
    let mut pn = ProcNames::default();
    assert!(pn.due(), "a fresh ProcNames is due immediately");
    pn.refresh(&[me]);
    assert!(!pn.due(), "after a refresh the throttle holds");
    assert!(pn.name(me).is_some(), "our own process should resolve");
}

#[test]
fn unknown_pid_is_none() {
    let mut pn = ProcNames::default();
    // PID 0 is never a real userland process to name.
    pn.refresh(&[]);
    assert_eq!(pn.name(0), None);
}

#[test]
fn a_command_line_is_the_program_by_name_then_its_arguments() {
    let argv = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        command_line(&argv(&[
            "/Users/me/.cargo/bin/cargo",
            "test",
            "-p",
            "crew-app"
        ]))
        .as_deref(),
        Some("cargo test -p crew-app")
    );
    assert_eq!(
        command_line(&argv(&["C:\\bin\\git.exe", "status"])).as_deref(),
        Some("git.exe status")
    );
    assert_eq!(command_line(&argv(&[])), None);
    assert_eq!(command_line(&argv(&[""])), None);
}

#[test]
fn our_own_command_line_is_known_after_a_refresh() {
    let me = std::process::id();
    let mut pn = ProcNames::default();
    pn.refresh(&[me]);
    assert!(pn.line(me).is_some_and(|l| !l.is_empty()));
    assert_eq!(
        pn.line_if(Some(me), &None),
        None,
        "nothing running, no line"
    );
}
