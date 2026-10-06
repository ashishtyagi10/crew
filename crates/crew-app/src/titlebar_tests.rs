use super::wanted;

#[test]
fn a_solid_window_leaves_the_native_title_bar_alone() {
    assert_eq!(wanted(1.0, (1, 2, 3), false, false), None);
}

#[test]
fn a_sheer_window_paints_its_title_bar_the_page_colour() {
    assert_eq!(
        wanted(0.88, (1, 2, 3), true, false),
        Some(([1, 2, 3], 1.0, true))
    );
    assert_eq!(
        wanted(crate::config::MIN_WINDOW_OPACITY, (9, 9, 9), true, false),
        Some(([9, 9, 9], 1.0, true))
    );
}

/// A light page gets a dark title whatever the OS is in: on a dark Mac
/// the bar crew painted light carried the system's white title.
#[test]
fn the_painted_bar_carries_the_page_s_own_ink() {
    assert_eq!(
        wanted(0.8, (250, 250, 250), false, false),
        Some(([250, 250, 250], 1.0, false))
    );
    assert_eq!(
        wanted(0.8, (10, 10, 10), true, false),
        Some(([10, 10, 10], 1.0, true))
    );
}

/// A tube's bar frosts with its window instead of drawing a black strip
/// across the top of the glass.
#[test]
fn a_tube_s_title_bar_is_as_sheer_as_its_page() {
    assert_eq!(
        wanted(0.12, (2, 6, 5), true, true),
        Some(([2, 6, 5], 0.12, true))
    );
}
