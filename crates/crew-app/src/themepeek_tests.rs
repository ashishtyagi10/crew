use super::*;

fn row(fill: &str) -> MenuItem {
    MenuItem {
        fill: fill.to_string(),
        ..Default::default()
    }
}

#[test]
fn arrowing_onto_a_palette_puts_it_on() {
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    for mode in ["dark", "light", "crt", "auto"] {
        assert!(!sync(&[row(&format!("/theme {mode}"))], 0), "{mode}");
        assert_eq!(crew_theme::current_id(), ThemeId::PaperDark, "{mode}");
    }
}

/// Every row that names no colour leaves everything alone.
#[test]
fn a_row_that_names_no_colour_previews_nothing() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    crew_theme::poleshift::set_custom(None);
    for fill in [
        "/theme",
        "/theme not-a-palette",
        "/gradient not-a-pair",
        // A LEVEL says how far the poles breathe, not which colours they are.
        "/gradient subtle",
        "/density roomy",
        "/out",
    ] {
        assert!(!sync(&[row(fill)], 0), "{fill}");
        assert_eq!(crew_theme::current_id(), ThemeId::PaperDark, "{fill}");
        assert_eq!(crew_theme::poleshift::custom(), None, "{fill}");
    }
}

/// A named gradient rides the same rule: a four-cell ramp beside a name is
/// not the light that pair puts on the canvas.
#[test]
fn arrowing_onto_a_gradient_puts_its_poles_on() {
    let _g = crate::app::theme_test_guard();
    crew_theme::poleshift::set_custom(None);
    let name = crew_theme::gradients::GRADIENTS[0].name;
    let want = crew_theme::gradients::by_name(name).expect("a named pair");
    assert!(sync(&[row(&format!("/gradient {name}"))], 0));
    assert_eq!(crew_theme::poleshift::custom(), Some(want));
    assert!(sync(&[], 0), "and leaving takes them off");
    assert_eq!(crew_theme::poleshift::custom(), None);
}

/// Walking from a palette row into a gradient row and out again restores the
/// PAIR you had, not just whichever one was previewed last.
#[test]
fn walking_between_two_pickers_restores_both() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::PaperLight);
    crew_theme::poleshift::set_custom(None);
    sync(&[row("/theme crt-green")], 0);
    let name = crew_theme::gradients::GRADIENTS[0].name;
    sync(&[row(&format!("/gradient {name}"))], 0);
    assert_ne!(crew_theme::poleshift::custom(), None);
    sync(&[], 0);
    assert_eq!(crew_theme::current_id(), ThemeId::PaperLight);
    assert_eq!(crew_theme::poleshift::custom(), None);
}

/// A chosen row forgets the preview WITHOUT undoing it: the command that runs
/// next sets the theme for real, and a restore on the way there would flash
/// the old palette back.
#[test]
fn accepting_keeps_the_previewed_palette() {
    let _g = crate::app::theme_test_guard();
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
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    sync(&[row("/theme crt-green")], 99);
    assert_eq!(crew_theme::current_id(), ThemeId::CrtGreen);
    sync(&[], 99);
}
