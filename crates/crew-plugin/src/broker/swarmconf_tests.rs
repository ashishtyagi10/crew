//! The sidecar is opt-in in every direction, and these are the directions: unset, set to
//! something that is not there, and set to something that is.
use super::*;

/// `CREW_SIDECAR` is read by nothing else, so setting it here races with no other test.
fn with_sidecar<T>(value: Option<&str>, f: impl FnOnce() -> T) -> T {
    match value {
        Some(v) => std::env::set_var("CREW_SIDECAR", v),
        None => std::env::remove_var("CREW_SIDECAR"),
    }
    let out = f();
    std::env::remove_var("CREW_SIDECAR");
    out
}

#[test]
fn no_sidecar_is_configured_by_default() {
    // The whole non-negotiable in one assertion: crew is a single binary with no runtime
    // dependency on anything, and nothing here changes that until somebody asks.
    assert_eq!(with_sidecar(None, sidecar_command), None);
}

#[test]
fn a_sidecar_that_is_not_installed_is_not_used() {
    // A machine with no Python must go on working exactly as it did — the goal's condition.
    assert_eq!(
        with_sidecar(Some("crew-no-such-engine --serve"), sidecar_command),
        None
    );
}

#[test]
fn a_sidecar_that_is_installed_is_read_as_a_command_and_its_arguments() {
    let program = if cfg!(windows) { "cmd" } else { "sh" };
    let line = format!("{program} -c true");
    let got = with_sidecar(Some(&line), sidecar_command);
    assert_eq!(
        got,
        Some((program.to_string(), vec!["-c".into(), "true".into()]))
    );
}

#[test]
fn a_blank_setting_is_no_sidecar_rather_than_an_empty_command() {
    assert_eq!(with_sidecar(Some("   "), sidecar_command), None);
}
