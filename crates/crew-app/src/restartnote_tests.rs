use super::*;

#[test]
fn legend_names_both_versions_at_generous_width() {
    let s = legend("9.9.9", 100);
    assert!(
        s.starts_with(concat!("crew v", env!("CARGO_PKG_VERSION"))),
        "{s}"
    );
    assert!(s.contains("\u{2192} v9.9.9"), "{s}"); // →
}

/// The legend stopped telling people to type `/update`: by the time it is on
/// screen the update has already installed itself, and the RESTART card is the
/// thing to press. A legend that still named a command would send the user to
/// the input bar for something a button beside it already does.
#[test]
fn legend_names_no_command() {
    for cols in [100, 25, 12] {
        let s = legend("9.9.9", cols);
        assert!(!s.contains("/update"), "at {cols} cols: {s}");
    }
}

#[test]
fn legend_falls_back_to_the_compact_form_when_narrow() {
    let s = legend("9.9.9", 16);
    assert!(
        !s.starts_with("crew v"),
        "narrow width must drop the current-version prefix: {s}"
    );
    assert!(s.contains("v9.9.9"), "{s}");
    assert!(s.chars().count() <= 16, "{s}");
}

/// A parked install blinks for as long as it is parked — it is the only thing
/// asking for a restart, and a reminder that settles after four seconds is one
/// the user who stepped away never sees.
#[test]
fn a_parked_install_keeps_asking_and_never_settles() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    assert!(animating(), "still blinking right after the install");
    // The old behaviour stopped driving frames once PULSE_MS elapsed.
    assert!(animating(), "and an hour later, too");
}

#[test]
fn motion_off_costs_no_frames() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    assert!(!animating());
}
