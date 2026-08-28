use super::*;

#[test]
fn the_ladder_reads_back_as_itself() {
    for (name, value, _) in LADDER {
        assert_eq!(parse(name), Some(*value), "/opacity {name}");
    }
}

/// Percent, percent sign, and fraction all name the same window — people
/// write it all three ways.
/// The named steps stay shy — a first `/opacity` must not drop the canvas
/// into the wallpaper. Anyone who wants that types a number.
#[test]
fn the_named_steps_are_a_texture_not_a_window() {
    for (name, value, _) in LADDER {
        assert!(
            *value >= 0.85,
            "/opacity {name} is {value}, sheerer than a named step should go"
        );
    }
    assert_eq!(parse("on"), parse("medium"), "`on` means the middle rung");
}

#[test]
fn a_number_is_read_the_way_it_was_written() {
    assert_eq!(parse("85"), Some(0.85));
    assert_eq!(parse("85%"), Some(0.85));
    assert_eq!(parse("0.85"), Some(0.85));
    assert_eq!(parse("100"), Some(1.0));
    assert_eq!(parse("1"), Some(1.0));
}

/// The floor is the point: a window dialled to nothing is a window you cannot
/// find again.
#[test]
fn a_window_can_never_be_dialled_away() {
    assert_eq!(parse("10"), Some(MIN_WINDOW_OPACITY));
    assert_eq!(parse("0"), Some(MIN_WINDOW_OPACITY));
    assert_eq!(parse("-40"), Some(MIN_WINDOW_OPACITY));
    assert_eq!(parse("400"), Some(1.0));
}

#[test]
fn nonsense_is_refused_rather_than_guessed() {
    assert_eq!(parse("glassy"), None);
    assert_eq!(parse(""), None);
    assert_eq!(parse("%"), None);
    assert_eq!(parse("nan"), None);
}

#[test]
fn percent_is_whole_numbers() {
    assert_eq!(percent(1.0), "100%");
    assert_eq!(percent(0.85), "85%");
    assert_eq!(percent(MIN_WINDOW_OPACITY), "35%");
}
