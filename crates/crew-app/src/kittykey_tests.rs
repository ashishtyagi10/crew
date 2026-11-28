//! Which key events reach the kitty encoders: a release only after the press
//! it answers.
use super::admit;
use winit::keyboard::KeyCode::{KeyA, KeyB};

#[test]
fn a_release_follows_only_a_press_the_program_saw() {
    assert!(admit(10, true, Some(KeyA)));
    assert!(admit(10, false, Some(KeyA)), "its release");
    assert!(!admit(10, false, Some(KeyA)), "once");
    // Cmd+B was crew's: no press reached the program, so no release goes.
    assert!(!admit(10, false, Some(KeyB)));
    assert!(
        !admit(10, false, None),
        "an unidentified key keeps no state"
    );
}

#[test]
fn flags_off_forget_every_key_held() {
    assert!(admit(2, true, Some(KeyA)));
    assert!(admit(0, true, Some(KeyB)), "a press always goes on");
    assert!(!admit(2, false, Some(KeyA)), "the program popped its flags");
}
