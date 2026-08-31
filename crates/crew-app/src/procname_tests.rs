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
