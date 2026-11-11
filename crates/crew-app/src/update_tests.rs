use super::*;
use crate::app::CrewApp;

#[test]
#[allow(clippy::field_reassign_with_default)] // test fixture: inject update state
fn loud_install_lingers_then_parks_without_restarting() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new(rx));
    let now = Instant::now();
    tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
    // First tick drains the install message and holds the card at "done".
    assert!(app.poll_update(now));
    assert!(matches!(app.update.as_ref().unwrap().stage, Stage::Done(_)));
    // Once the card's linger elapses it clears — and that is all. Restarting
    // is `/restart`'s, even for an update the user typed.
    assert!(app.poll_update(now + NOTE_TTL));
    assert!(app.update.is_none());
    assert!(!app.want_restart, "an install never restarts by itself");
    assert!(app.parked_update.is_some());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn silent_install_parks_and_never_asks_to_restart() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = CrewApp::default();
    app.update = Some(UpdateState::new_with(rx, true));
    tx.send(UpdateMsg::Installed("9.9.9".into())).unwrap();
    let now = Instant::now();
    app.poll_update(now);
    app.poll_update(now);
    assert!(!app.want_restart, "a background install must not interrupt");
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
    app.poll_update(now + NOTE_TTL);
    assert!(app.update.is_none());
    assert!(
        app.parked_update.is_none(),
        "nothing installed, nothing parked"
    );
}

#[test]
fn update_cmd_action_prefers_parked_then_takeover_then_refusal() {
    use super::UpdateCmdAction::*;
    // A parked install wins over everything — even an in-flight run.
    assert_eq!(update_cmd_action(true, None), Parked);
    assert_eq!(update_cmd_action(true, Some((true, true))), Parked);
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
            redrew |= app.poll_update(now);
        }
        assert_eq!(
            redrew, !silent,
            "silent={silent}: animation-tick redraw must only fire when loud"
        );
    }
}
