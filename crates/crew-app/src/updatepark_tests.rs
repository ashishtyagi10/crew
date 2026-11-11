//! What an install leaves behind once it lands: the parked version, and the
//! one note that says so.
use super::*;
use crate::app::CrewApp;

/// "A note should appear": the moment an install lands, a toast and the
/// status line both name the version and the command — once per version.
#[test]
#[allow(clippy::field_reassign_with_default)]
fn an_install_landing_says_so_once() {
    let mut app = CrewApp::default();
    for round in 0..2 {
        let (tx, rx) = std::sync::mpsc::channel();
        app.update = Some(UpdateState::new_with(rx, true));
        tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
        app.status = None;
        app.poll_update(Instant::now());
        let said = app.status.clone().map(|s| s.0);
        match round {
            0 => {
                let s = said.expect("the first landing is announced");
                assert!(s.contains("v9.9.9") && s.contains("/restart"), "{s}");
            }
            _ => assert!(said.is_none(), "the same version is not re-announced"),
        }
        assert_eq!(app.toasts.len(), 1, "round {round}");
    }
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn installed_parks_the_update_version_in_both_modes() {
    for silent in [true, false] {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = CrewApp::default();
        app.update = Some(UpdateState::new_with(rx, silent));
        tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
        app.poll_update(Instant::now());
        assert_eq!(
            app.parked_update.as_ref().map(|(v, _)| v.as_str()),
            Some("9.9.9"),
            "silent={silent}"
        );
    }
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn reparks_when_a_second_install_lands_a_different_version() {
    let mut app = CrewApp::default();
    // Baseline against the live clock, not a literal: `anim::now_ms()` is
    // ms-since-first-call, so a hardcoded stale stamp could race a fresh
    // clock in the first milliseconds of the test process.
    let baseline = crate::anim::now_ms();
    let stale_stamp = baseline.saturating_sub(10);
    app.parked_update = Some(("1.0.1".into(), stale_stamp));

    let (tx, rx) = std::sync::mpsc::channel();
    app.update = Some(UpdateState::new_with(rx, true));
    tx.send(UpdateMsg::Installed("1.0.2".into())).unwrap();
    app.poll_update(Instant::now());
    let (v, at) = app.parked_update.clone().expect("still parked");
    assert_eq!(v, "1.0.2", "legend updates to the newly installed version");
    assert!(
        at >= baseline,
        "stamp refreshes so the blink pulse re-fires"
    );

    // A second Done for the SAME version must not re-stamp (no repeat nag).
    let (tx2, rx2) = std::sync::mpsc::channel();
    app.update = Some(UpdateState::new_with(rx2, true));
    tx2.send(UpdateMsg::Installed("1.0.2".into())).unwrap();
    app.poll_update(Instant::now());
    let (v2, at2) = app.parked_update.clone().expect("still parked");
    assert_eq!(v2, "1.0.2");
    assert_eq!(at2, at, "same-version reinstall does not re-stamp");
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn auto_update_is_a_noop_while_an_install_is_parked() {
    let mut app = CrewApp::default();
    app.parked_update = Some(("9.9.9".into(), 0));
    app.start_auto_update();
    assert!(
        app.update.is_none(),
        "parked install suppresses further auto checks"
    );
}
