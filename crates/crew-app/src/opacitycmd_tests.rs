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

/// On glass the window is the theme's (25%): `/opacity 60` says it saved
/// 60% and what the window is, not "the title bar stays solid" — glass's
/// bar frosts with its panes. Elsewhere the reply is as it was.
#[test]
fn on_glass_the_reply_says_the_theme_sets_the_window() {
    let _g = crate::app::theme_test_guard();
    let said = |app: &CrewApp| app.log.last().map(|e| e.text.clone()).unwrap_or_default();
    crew_theme::set_theme(crew_theme::ThemeId::GlassClear);
    let mut app = CrewApp::default();
    app.opacity_command("60");
    assert!(
        said(&app).ends_with("opacity 60% saved — glass sets the window to 25%"),
        "{}",
        said(&app)
    );
    app.opacity_command("");
    assert!(
        said(&app).contains("glass sets the window to 25%"),
        "{}",
        said(&app)
    );
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    app.opacity_command("60");
    assert!(
        said(&app).contains("the title bar stays solid"),
        "{}",
        said(&app)
    );
}

/// On glass the picker heads its ladder with what the window is — the
/// steps apply elsewhere; on a paper theme it is the ladder alone.
#[test]
fn the_picker_says_when_the_steps_do_nothing_here() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::GlassClear);
    let rows = picker();
    assert_eq!(rows[0].0, "", "a heading, not a choice");
    assert!(
        rows[0].1.starts_with("glass sets the window to 25%"),
        "{}",
        rows[0].1
    );
    assert_eq!(rows.len(), LADDER.len() + 1);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert_eq!(picker().len(), LADDER.len());
    assert!(picker().iter().all(|(v, _)| !v.is_empty()));
}
