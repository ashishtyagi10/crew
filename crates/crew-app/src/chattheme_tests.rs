use super::*;
use crew_theme::ThemeId;

#[test]
fn parse_no_arg_lists() {
    assert_eq!(parse_theme_cmd(""), ThemeCmd::List);
    assert_eq!(parse_theme_cmd("   "), ThemeCmd::List);
}

#[test]
fn parse_known_name_switches() {
    assert_eq!(
        parse_theme_cmd("paper-light"),
        ThemeCmd::Select(crew_theme::Selection::Fixed(ThemeId::PaperLight))
    );
    assert_eq!(
        parse_theme_cmd(" crt-green "),
        ThemeCmd::Select(crew_theme::Selection::Fixed(ThemeId::CrtGreen))
    );
}

#[test]
fn parse_unknown_name_is_unknown() {
    assert_eq!(parse_theme_cmd("nope"), ThemeCmd::Unknown("nope".into()));
}

#[test]
fn parse_modes_and_alias() {
    // The three canonical names.
    assert_eq!(
        parse_theme_cmd("dark"),
        ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Dark))
    );
    assert_eq!(
        parse_theme_cmd("light"),
        ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Light))
    );
    assert_eq!(
        parse_theme_cmd(" CRT "),
        ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Crt))
    );
    // Pre-consolidation names still parse.
    assert_eq!(
        parse_theme_cmd("random"),
        ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Dark))
    );
    assert_eq!(
        parse_theme_cmd("random-light"),
        ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Light))
    );
    assert_eq!(
        parse_theme_cmd(" AUTO "),
        ThemeCmd::Select(crew_theme::Selection::Mode(crew_theme::RandomMode::Auto))
    );
}

#[test]
fn list_line_names_the_four_modes_and_marks_the_active_one() {
    let line = theme_list_line(None);
    for m in crew_theme::THEME_MODES {
        assert!(line.contains(m.as_str()), "missing {}: {line}", m.as_str());
        assert!(line.contains(m.describe()), "missing desc: {line}");
    }
    // Nothing is marked while no mode is on.
    assert!(
        !line.contains("\u{25cf}"),
        "nothing should be marked: {line}"
    );
    // The pooled palettes are not listed as entries.
    assert!(
        !line.contains("paper-dark") && !line.contains("crt-green"),
        "individual palettes must not be listed: {line}"
    );
}

#[test]
fn list_line_marks_the_active_mode() {
    let line = theme_list_line(Some(crew_theme::RandomMode::Light));
    assert!(
        line.contains("\u{25cf} light"),
        "light mode not marked: {line}"
    );
    assert!(!line.contains("\u{25cf} dark"), "wrong mode marked: {line}");
}

#[test]
fn theme_names_lists_the_four_modes() {
    let names = theme_names();
    for m in crew_theme::THEME_MODES {
        assert!(
            names.contains(m.as_str()),
            "missing {}: {names}",
            m.as_str()
        );
    }
    // Legacy/pooled names are not advertised (they still parse).
    assert!(
        !names.contains("random-dark") && !names.contains("paper-dark"),
        "legacy names must not be listed: {names}"
    );
}

#[test]
fn intercept_distinguishes_switch_list_and_foreign_text() {
    let _g = crate::app::theme_test_guard();
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    assert_eq!(
        intercept(&mut p, "/theme paper-dark"),
        ThemeIntercept::Switched
    );
    assert_eq!(intercept(&mut p, "/theme"), ThemeIntercept::Handled);
    assert_eq!(intercept(&mut p, "/theme nope"), ThemeIntercept::Handled);
    assert_eq!(intercept(&mut p, "hello"), ThemeIntercept::NotTheme);
}

#[test]
fn switch_after_random_clears_random_mode() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Mode(crew_theme::RandomMode::Dark),
        1_000,
    );
    assert!(crew_theme::is_random());
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        2_000,
    );
    assert!(!crew_theme::is_random());
    assert_eq!(crew_theme::current_id(), ThemeId::PaperDark);
}
