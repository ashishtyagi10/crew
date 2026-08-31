use super::*;

/// The mode is only worth having if it actually deepens the wash, and
/// only usable if it stops short of hiding the rest of the grid.
#[test]
fn focus_deepens_the_spotlight_without_erasing_it() {
    let _g = crate::app::motion_test_guard();
    set(false);
    assert_eq!(dim(), crate::spotlight::DIM);
    set(true);
    assert_eq!(dim(), DIM);
    // Compile-time: these are the contract on the constant itself, not on
    // any run of the code.
    const _: () = assert!(DIM > crate::spotlight::DIM);
    const _: () = assert!(DIM < 0.6);
    set(false);
}

/// Zero held is not news. Anything else has to be counted exactly — the
/// whole promise of holding over dropping is that the number is true.
#[test]
fn the_summary_counts_and_stays_quiet_at_zero() {
    assert_eq!(Held::default().summary(), None);
    assert_eq!(
        Held { toasts: 1 }.summary().as_deref(),
        Some("1 notification held while focused")
    );
    assert_eq!(
        Held { toasts: 4 }.summary().as_deref(),
        Some("4 notifications held while focused")
    );
}
