use crate::app::CrewApp;

#[test]
fn weight_defaults_to_semibold_and_named_steps_set_it() {
    let mut app = CrewApp::default();
    assert_eq!(app.config.font_weight, 600, "SemiBold out of the box");
    app.weight_command("bold");
    assert_eq!(app.config.font_weight, 700);
    app.weight_command("medium");
    assert_eq!(app.config.font_weight, 500);
    app.weight_command("black");
    assert_eq!(app.config.font_weight, 900);
}

#[test]
fn weight_accepts_a_raw_number_clamped_to_range() {
    let mut app = CrewApp::default();
    app.weight_command("650");
    assert_eq!(app.config.font_weight, 650);
    app.weight_command("5000"); // clamps
    assert_eq!(app.config.font_weight, 900);
}

#[test]
fn weight_bad_arg_leaves_it_untouched() {
    let mut app = CrewApp::default();
    app.weight_command("bold");
    app.weight_command("chunky");
    assert_eq!(
        app.config.font_weight, 700,
        "bad arg must not change weight"
    );
}

#[test]
fn smooth_defaults_on_and_named_steps_set_it() {
    let mut app = CrewApp::default();
    assert_eq!(
        app.config.font_smooth,
        crew_render::DEFAULT_SMOOTH,
        "the darkening is off out of the box — the coverage curve \
         delivers the outline's light on its own"
    );
    app.smooth_command("heavy");
    assert_eq!(app.config.font_smooth, 120);
    app.smooth_command("light");
    assert_eq!(app.config.font_smooth, 40);
    app.smooth_command("medium");
    assert_eq!(app.config.font_smooth, 70);
    app.smooth_command("off");
    assert_eq!(app.config.font_smooth, crew_render::DEFAULT_SMOOTH);
}

#[test]
fn smooth_accepts_a_raw_number_clamped_to_a_byte() {
    let mut app = CrewApp::default();
    app.smooth_command("42");
    assert_eq!(app.config.font_smooth, 42);
    app.smooth_command("9000"); // clamps
    assert_eq!(app.config.font_smooth, 255);
}

/// A Settings-form save routes its config through `apply_settings`; the
/// smoothing it carries must land on `app.config` — the key `/smooth`
/// then reads — or the form's picker would look editable while changing
/// nothing. Fails if the apply path (or `clamped()`) drops `font_smooth`.
#[test]
fn settings_apply_adopts_the_forms_smoothing() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let mut pane = crate::settingspane::SettingsPane::new(app.config.clone(), Vec::new());
    pane.focus = crate::settingspane::FIELDS
        .iter()
        .position(|&f| f == crate::settingspane::Field::Smooth)
        .unwrap();
    crate::settingspane::cycle_value(&mut pane, false); // off → light
    let crate::settingspane::SettingsAction::Apply(cfg) = pane.save() else {
        panic!("save must apply");
    };
    app.apply_settings(*cfg);
    assert_eq!(app.config.font_smooth, 40);
    app.smooth_command("");
    let s = app.active_status().unwrap();
    assert!(s.contains("light"), "/smooth reports the form's value: {s}");
}

#[test]
fn smooth_bad_arg_leaves_it_untouched() {
    let mut app = CrewApp::default();
    app.smooth_command("120");
    app.smooth_command("glassy");
    assert_eq!(
        app.config.font_smooth, 120,
        "bad arg must not change smoothing"
    );
}

#[test]
fn gamma_keywords_and_numbers_both_land() {
    let mut app = CrewApp::default();
    app.gamma_command("off");
    assert_eq!(app.config.font_gamma, 0);
    app.gamma_command("full");
    assert_eq!(app.config.font_gamma, crew_render::DEFAULT_TEXT_GAMMA);
    app.gamma_command("medium");
    assert_eq!(app.config.font_gamma, 130);
    app.gamma_command("42");
    assert_eq!(app.config.font_gamma, 42);
    app.gamma_command("9000"); // clamps
    assert_eq!(app.config.font_gamma, 255);
}

#[test]
fn gamma_bad_arg_leaves_it_untouched() {
    let mut app = CrewApp::default();
    app.gamma_command("light");
    app.gamma_command("chunky");
    assert_eq!(app.config.font_gamma, 65, "bad arg must not change gamma");
}

/// Same parity the smoothing picker keeps: a Settings save must land on
/// the key `/gamma` reads, or the form would look editable while
/// changing nothing.
#[test]
fn settings_apply_adopts_the_forms_text_gamma() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let mut pane = crate::settingspane::SettingsPane::new(app.config.clone(), Vec::new());
    pane.focus = crate::settingspane::FIELDS
        .iter()
        .position(|&f| f == crate::settingspane::Field::FontGamma)
        .unwrap();
    crate::settingspane::cycle_value(&mut pane, false); // full → off
    let crate::settingspane::SettingsAction::Apply(cfg) = pane.save() else {
        panic!("save must apply");
    };
    app.apply_settings(*cfg);
    assert_eq!(app.config.font_gamma, 0);
    app.gamma_command("");
    let s = app.active_status().unwrap();
    assert!(s.contains("off"), "/gamma reports the form's value: {s}");
}
