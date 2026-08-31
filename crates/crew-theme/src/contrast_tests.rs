use super::*;

/// The switch has to actually move things, and in the direction the user
/// asked. A floor pair that did not rise, or an effect that did not back
/// off, is the feature shipped as a no-op.
#[test]
fn asking_for_contrast_raises_the_floors_and_quiets_the_effects() {
    let _g = test_lock();
    set_high_contrast(false);
    let (t0, m0, e0) = (text_floor(), mark_floor(), effect_scale());
    set_high_contrast(true);
    let (t1, m1, e1) = (text_floor(), mark_floor(), effect_scale());
    set_high_contrast(false);

    assert!(t1 > t0, "text floor {t0} -> {t1}");
    assert!(m1 > m0, "mark floor {m0} -> {m1}");
    assert!(e1 < e0, "effects {e0} -> {e1}");
    // The ordinary floors must stay exactly the WCAG AA bands crew's
    // whole derivation contract is written against.
    assert_eq!((t0, m0, e0), (4.5, 3.0, 1.0));
    // And the raised ones must be the standard's next band, not a number
    // someone liked the look of.
    assert_eq!((t1, m1), (7.0, 4.5));
}

/// A mark never has to clear more than text does — that ordering is what
/// makes the two bands mean anything.
#[test]
fn a_mark_is_never_asked_for_more_than_text() {
    let _g = test_lock();
    for on in [false, true] {
        set_high_contrast(on);
        assert!(mark_floor() <= text_floor(), "at high={on}");
    }
    set_high_contrast(false);
}

/// The spotlight must not be switched off: it is the cue that says which
/// pane has focus, and losing it is itself an accessibility regression.
#[test]
fn the_effects_are_quieted_not_killed() {
    let _g = test_lock();
    set_high_contrast(true);
    assert!(effect_scale() > 0.0);
    set_high_contrast(false);
}
