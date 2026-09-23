use super::*;

const WIN: (f32, f32) = (1000.0, 500.0);

/// The tilt spans the window: the right edge is +1 across, the top -1 down.
#[test]
fn the_aim_maps_the_window_onto_the_tilt() {
    let mut l = PointerLight::default();
    assert!(l.aim(Some((1000.0, 0.0)), WIN));
    l.step(10_000, MotionLevel::Full);
    assert_eq!(l.tilt(), (1.0, -1.0));
    assert!(!l.moving(), "a glide that arrived asks for no more frames");
}

/// A pointer jittering inside one step asks for no frame at all; a move of a
/// whole step does.
#[test]
fn only_a_whole_step_asks_for_a_frame() {
    let mut l = PointerLight::default();
    l.aim(Some((500.0, 250.0)), WIN);
    l.step(10_000, MotionLevel::Full);
    assert!(!l.aim(Some((502.0, 251.0)), WIN), "a pixel of jitter");
    assert!(l.aim(Some((600.0, 250.0)), WIN), "a tenth of the window");
}

/// The light glides — part of the way after one frame, there in the end —
/// and snaps at once with motion off.
#[test]
fn the_tilt_glides_unless_motion_is_off() {
    let mut l = PointerLight::default();
    l.aim(Some((1000.0, 250.0)), WIN);
    l.step(16, MotionLevel::Full);
    let x = l.tilt().0;
    assert!(x > 0.0 && x < 1.0, "part of the way: {x}");
    assert!(l.moving());
    let mut off = PointerLight::default();
    off.aim(Some((1000.0, 250.0)), WIN);
    off.step(16, MotionLevel::Off);
    assert_eq!(off.tilt(), (1.0, 0.0));
}

/// A pointer that leaves takes the light home to its resting direction.
#[test]
fn leaving_the_window_sends_the_light_home() {
    let mut l = PointerLight::default();
    l.aim(Some((0.0, 0.0)), WIN);
    l.step(10_000, MotionLevel::Full);
    assert!(l.aim(None, WIN));
    l.step(10_000, MotionLevel::Full);
    assert_eq!(l.tilt(), (0.0, 0.0));
}
