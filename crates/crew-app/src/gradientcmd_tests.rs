use super::*;

use crew_theme::poleshift;

/// `/gradient` writes process-wide globals (the ladder, the custom pair), so
/// every test here serialises against the app's theme guard and puts back what
/// it found.
fn restore(level: GradientLevel, custom: Option<Poles>) {
    crate::gradientlvl::set_level(level);
    poleshift::set_custom(custom);
}

#[test]
fn a_pair_round_trips_through_the_stored_form() {
    for pair in [
        ((0, 0, 0), (255, 255, 255)),
        ((122, 162, 247), (187, 154, 247)),
        ((1, 2, 3), (253, 254, 255)),
    ] {
        assert_eq!(parse_poles(&format_poles(pair)), Some(pair), "{pair:?}");
    }
}

/// Both separators people actually type, and the `#` being optional — the
/// same latitude `Accent (#hex)` in Settings gives.
#[test]
fn the_parser_takes_the_forms_people_type() {
    let want = Some(((17, 34, 51), (68, 85, 102)));
    for s in [
        "#112233 #445566",
        "112233 445566",
        "  #112233,#445566  ",
        "#112233,  445566",
    ] {
        assert_eq!(parse_poles(s), want, "{s:?}");
    }
}

/// A half-understood gradient must be refused outright rather than half
/// applied — the canvas would otherwise end up in a state nobody asked for.
#[test]
fn a_partial_or_malformed_pair_is_refused() {
    for s in [
        "",
        "#112233",                 // one colour is not a gradient
        "#112233 #445566 #778899", // three is not either
        "#112233 #44556",          // five digits
        "#112233 #zzzzzz",         // not hex
        "red blue",                // names are not supported
        "#112233 subtle",          // a level is not a colour
    ] {
        assert_eq!(parse_poles(s), None, "{s:?} should not parse");
    }
}

/// The ladder arm: `/gradient lively` is the same setting the Settings picker
/// writes, so the two can never disagree about what the canvas is doing.
#[test]
fn the_command_sets_the_same_ladder_the_form_does() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    app.gradient_command("lively");
    assert_eq!(app.config.gradient, "lively");
    assert_eq!(app.config.gradient_level(), GradientLevel::Lively);
    assert_eq!(crate::gradientlvl::level(), GradientLevel::Lively);
    app.gradient_command("off");
    assert_eq!(crate::gradientlvl::level(), GradientLevel::Off);
    // Off must also unwind a lean the last breath left in the global.
    assert_eq!(poleshift::shift(), 0.0);
    restore(l0, c0);
}

/// A custom pair reaches the canvas, is persisted in a form that reads back,
/// and `reset` takes it away again.
#[test]
fn a_custom_pair_is_adopted_and_can_be_dropped() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    app.gradient_command("#dd2828 #2856dd");
    assert_eq!(
        app.config.gradient_poles.as_deref(),
        Some("#dd2828 #2856dd")
    );
    assert_eq!(poleshift::custom(), Some(((221, 40, 40), (40, 86, 221))));
    app.gradient_command("reset");
    assert_eq!(app.config.gradient_poles, None);
    assert_eq!(poleshift::custom(), None);
    restore(l0, c0);
}

/// The colour is the user's; the brightness is not. Typing white must not
/// bleach the page — the wash lies under the text and has almost no contrast
/// headroom to spend.
#[test]
fn a_white_pair_does_not_bleach_the_page() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    let before = poleshift::poles().expect("every theme ships poles");
    app.gradient_command("#ffffff #ffffff");
    let after = poleshift::poles().expect("every theme ships poles");
    let lum = |(r, g, b): (u8, u8, u8)| {
        0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)
    };
    assert!(
        (lum(after.0) - lum(before.0)).abs() < 24.0,
        "white poles moved the wash's brightness: {before:?} -> {after:?}"
    );
    restore(l0, c0);
}

/// Garbage leaves everything exactly as it was and says so — a typo must not
/// change the canvas.
#[test]
fn a_bad_argument_changes_nothing() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    app.gradient_command("subtle");
    let (level, poles) = (
        app.config.gradient.clone(),
        app.config.gradient_poles.clone(),
    );
    app.gradient_command("#nothex");
    assert_eq!(app.config.gradient, level);
    assert_eq!(app.config.gradient_poles, poles);
    let said = app.active_status().unwrap_or("");
    assert!(said.contains("usage:"), "a typo must say so, got {said:?}");
    restore(l0, c0);
}

/// `apply_gradient` is the one push from config to globals, so a config
/// arriving from session restore or an external edit lands the same as the
/// command's own path.
#[test]
fn applying_the_config_pushes_both_halves() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    app.config.gradient = "lively".to_string();
    app.config.gradient_poles = Some("#112233 #445566".to_string());
    app.apply_gradient();
    assert_eq!(crate::gradientlvl::level(), GradientLevel::Lively);
    assert_eq!(poleshift::custom(), Some(((17, 34, 51), (68, 85, 102))));
    // A pair that cannot be read is no pair — better the theme's own gradient
    // than a half-applied one.
    app.config.gradient_poles = Some("nonsense".to_string());
    app.apply_gradient();
    assert_eq!(poleshift::custom(), None);
    restore(l0, c0);
}

/// A name off the shelf is a gradient: `/gradient ember` reaches the canvas,
/// and is stored under the NAME so the preset can be re-tuned later without
/// everyone who chose it keeping the old colours.
#[test]
fn a_named_gradient_is_stored_by_name() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    app.gradient_command("ember");
    assert_eq!(app.config.gradient_poles.as_deref(), Some("ember"));
    assert_eq!(
        poleshift::custom(),
        crew_theme::gradients::by_name("ember"),
        "the named pair must reach the canvas"
    );
    // …and it reads back the same way a config from disk would.
    app.apply_gradient();
    assert_eq!(poleshift::custom(), crew_theme::gradients::by_name("ember"));
    restore(l0, c0);
}

/// Every name the value picker offers actually runs — a row that did nothing
/// would look exactly like a row whose arm was deleted.
#[test]
fn every_offered_name_runs() {
    let _g = crate::app::theme_test_guard();
    let (l0, c0) = (crate::gradientlvl::level(), poleshift::custom());
    let mut app = crate::app::CrewApp::default();
    for g in crew_theme::gradients::GRADIENTS {
        app.gradient_command(g.name);
        assert_eq!(poleshift::custom(), Some(g.poles), "/gradient {}", g.name);
    }
    restore(l0, c0);
}

/// A name and a hex pair cannot collide — no name is six hex digits — so the
/// parser can take names first without shadowing a colour anyone could type.
#[test]
fn no_name_could_be_mistaken_for_a_colour() {
    for g in crew_theme::gradients::GRADIENTS {
        assert!(
            crate::palette::parse_hex(g.name).is_none(),
            "{} parses as a colour",
            g.name
        );
    }
}
