use super::*;
use crate::app::CrewApp;

#[test]
#[allow(clippy::field_reassign_with_default)] // test fixture: inject update state
fn loud_install_shows_the_beat_then_asks_to_restart() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new(rx));
    let now = Instant::now();
    tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
    // First tick drains the install message and parks the card at "done".
    let tick = app.poll_update(now);
    assert!(tick.redraw);
    assert!(!tick.restart, "the restarting… beat shows first");
    assert!(matches!(app.update.as_ref().unwrap().stage, Stage::Done(_)));
    // Once the beat elapses the tick asks for the restart and clears.
    let tick = app.poll_update(now + RESTART_DELAY);
    assert!(tick.restart, "loud install rides into the new build");
    assert!(app.update.is_none());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn silent_install_parks_and_never_asks_to_restart() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new_with(rx, true));
    tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
    let now = Instant::now();
    let tick = app.poll_update(now);
    assert!(!tick.restart, "a background install must not interrupt");
    let tick = app.poll_update(now);
    assert!(!tick.restart);
    assert!(app.update.is_none(), "cleared, waiting parked");
    assert!(app.parked_update.is_some());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn loud_up_to_date_clears_without_a_restart() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new(rx));
    let now = Instant::now();
    tx.send(UpdateMsg::UpToDate("1.0.0".into())).unwrap();
    app.poll_update(now);
    let tick = app.poll_update(now + NOTE_TTL);
    assert!(!tick.restart, "nothing installed, nothing to restart into");
    assert!(app.update.is_none());
}

#[test]
fn update_cmd_action_prefers_parked_then_takeover_then_refusal() {
    use super::UpdateCmdAction::*;
    // A parked install wins over everything — even an in-flight run.
    assert_eq!(update_cmd_action(true, None), RestartParked);
    assert_eq!(update_cmd_action(true, Some((true, true))), RestartParked);
    // Silent run: taken over loudly, whether animating or settled.
    assert_eq!(update_cmd_action(false, Some((true, true))), TakeoverSilent);
    assert_eq!(
        update_cmd_action(false, Some((true, false))),
        TakeoverSilent
    );
    // Loud + animating: refuse the duplicate. Loud + settled: respawn.
    assert_eq!(
        update_cmd_action(false, Some((false, true))),
        AlreadyRunning
    );
    assert_eq!(update_cmd_action(false, Some((false, false))), Spawn);
    assert_eq!(update_cmd_action(false, None), Spawn);
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
fn silent_terminal_notes_clear_without_lingering() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new_with(rx, true));
    tx.send(UpdateMsg::UpToDate("1.0.0".into())).unwrap();
    let now = Instant::now();
    app.poll_update(now);
    // Silent up-to-date does NOT park a 5s note card — cleared within the
    // same poll_update call that drains the message (deadline == now).
    app.poll_update(now);
    assert!(
        app.update.is_none(),
        "silent terminal state must not linger"
    );
    assert!(app.parked_update.is_none());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn manual_update_upgrades_a_silent_run_to_loud() {
    let (_tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new_with(rx, true));
    app.start_update();
    let u = app.update.as_ref().unwrap();
    assert!(!u.silent, "manual /update takes over the silent run loudly");
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn silent_animation_ticks_do_not_redraw_but_loud_does() {
    for silent in [true, false] {
        let (_tx, rx) = std::sync::mpsc::channel();
        let mut app = CrewApp::default();
        app.update = Some(UpdateState::new_with(rx, silent));
        let now = Instant::now();
        let mut redrew = false;
        // Enough ticks to cross the SPINNER_DIV frame boundary at least once.
        for _ in 0..(SPINNER_DIV as usize + 1) {
            let tick = app.poll_update(now);
            redrew |= tick.redraw;
        }
        assert_eq!(
            redrew, !silent,
            "silent={silent}: animation-tick redraw must only fire when loud"
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
