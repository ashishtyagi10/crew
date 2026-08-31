use crate::app::CrewApp;

#[test]
fn crt_on_off_auto_set_the_override() {
    let mut app = CrewApp::default();
    assert_eq!(app.config.crt, None, "defaults to following the theme");
    app.crt_command("on");
    assert_eq!(app.config.crt, Some(true));
    app.crt_command("off");
    assert_eq!(app.config.crt, Some(false));
    app.crt_command("auto");
    assert_eq!(app.config.crt, None);
}

#[test]
fn bare_crt_toggles_the_effective_state() {
    let mut app = CrewApp::default();
    // A paper theme is CRT-off by default, so the first bare toggle pins on.
    let before = app.effective_crt().is_some();
    app.crt_command("");
    assert_eq!(app.config.crt, Some(!before));
    app.crt_command("");
    assert_eq!(app.config.crt, Some(before));
}

#[test]
fn crt_unknown_arg_leaves_state_untouched() {
    let mut app = CrewApp::default();
    app.crt_command("on");
    app.crt_command("wobble");
    assert_eq!(app.config.crt, Some(true), "bad arg must not change state");
}

/// `/motion` persists a preference, not a resolved strength — storing
/// `off` for an auto user would freeze them at whatever the OS happened to
/// say the day they ran the command.
#[test]
fn motion_command_stores_the_preference_and_publishes_the_level() {
    use crate::motion::{MotionLevel, MotionPref};
    let _g = crate::app::motion_test_guard();
    let mut app = CrewApp::default();
    assert_eq!(app.config.motion_pref(), MotionPref::Auto, "fresh default");

    app.motion_command("subtle");
    assert_eq!(app.config.motion, "subtle");
    assert_eq!(crate::motion::level(), MotionLevel::Subtle);

    app.motion_command("auto");
    assert_eq!(app.config.motion, "auto", "auto is stored as auto");
    crate::motion::set_os_reduce(true);
    assert_eq!(
        app.config.motion_level(),
        MotionLevel::Off,
        "auto must re-resolve when the OS switch flips"
    );
    crate::motion::set_os_reduce(false);
    assert_eq!(app.config.motion_level(), MotionLevel::Full);

    // A typo must change nothing.
    app.motion_command("swooshy");
    assert_eq!(app.config.motion, "auto");
    crate::motion::set_level(MotionLevel::Full);
}
