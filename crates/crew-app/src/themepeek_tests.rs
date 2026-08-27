use super::*;

fn row(fill: &str) -> MenuItem {
    MenuItem {
        fill: fill.to_string(),
        ..Default::default()
    }
}

/// Serialises with everything else that moves the process-wide theme.
fn guard() -> crate::app::ThemeGuard {
    crate::app::theme_test_guard()
}

#[test]
fn arrowing_onto_a_palette_puts_it_on() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    let menu = [row("/theme crt-green")];
    assert!(sync(&menu, 0), "the screen changed");
    assert_eq!(crew_theme::current_id(), ThemeId::CrtGreen);
    // Settling on the same row again is not a change, so it costs no repaint.
    assert!(!sync(&menu, 0));
    sync(&[], 0);
}

/// Leaving the row takes it off, and the one you had is exactly the one you
/// get back — the case a preview has to get right or it is a way to lose your
/// theme.
#[test]
fn leaving_the_picker_puts_the_real_theme_back() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperLight);
    sync(&[row("/theme crt-green")], 0);
    assert_eq!(crew_theme::current_id(), ThemeId::CrtGreen);
    assert!(sync(&[], 0), "the screen changed back");
    assert_eq!(crew_theme::current_id(), ThemeId::PaperLight);
    assert!(!sync(&[], 0), "and there is nothing left to restore");
}

/// The SECOND arrow key must not record the first preview as the thing to go
/// back to — walking the whole list and pressing Esc has to land where you
/// started, not on the eleventh palette.
#[test]
fn walking_the_list_still_restores_where_you_started() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    for id in crew_theme::ALL_THEMES {
        sync(&[row(&format!("/theme {}", id.as_str()))], 0);
        assert_eq!(crew_theme::current_id(), id);
    }
    sync(&[], 0);
    assert_eq!(crew_theme::current_id(), ThemeId::PaperDark);
}

/// A rotation mode names a POOL, and the palette it would land on is a choice
/// crew makes later — previewing "one of these four" by picking one would show
/// something the choice does not promise.
#[test]
fn a_rotation_mode_is_not_previewed() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    for mode in ["dark", "light", "crt", "auto"] {
        assert!(!sync(&[row(&format!("/theme {mode}"))], 0), "{mode}");
        assert_eq!(crew_theme::current_id(), ThemeId::PaperDark, "{mode}");
    }
}

/// Every other row in the palette leaves the theme alone — including the rows
/// of the other pickers, which also fill a `/command value` string.
#[test]
fn nothing_but_the_theme_picker_previews_anything() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    for fill in [
        "/theme",
        "/theme not-a-palette",
        "/gradient aurora",
        "/density roomy",
        "/out",
    ] {
        assert!(!sync(&[row(fill)], 0), "{fill}");
        assert_eq!(crew_theme::current_id(), ThemeId::PaperDark, "{fill}");
    }
}

/// A chosen row forgets the preview WITHOUT undoing it: the command that runs
/// next sets the theme for real, and a restore on the way there would flash
/// the old palette back.
#[test]
fn accepting_keeps_the_previewed_palette() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    sync(&[row("/theme crt-green")], 0);
    accept();
    assert_eq!(crew_theme::current_id(), ThemeId::CrtGreen);
    assert!(!sync(&[], 0), "there is nothing to restore");
    assert_eq!(crew_theme::current_id(), ThemeId::CrtGreen);
}

/// A selection past the end of the menu is the last row, not a panic — the
/// menu is rebuilt from the text on every frame and can shrink under a
/// selection that has not been reset yet.
#[test]
fn a_selection_past_the_end_lands_on_the_last_row() {
    let _g = guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    sync(&[row("/theme crt-green")], 99);
    assert_eq!(crew_theme::current_id(), ThemeId::CrtGreen);
    sync(&[], 99);
}
