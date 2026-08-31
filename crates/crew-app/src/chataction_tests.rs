use super::*;

#[test]
fn persist_theme_saves_the_live_mode_name() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Mode(crew_theme::RandomMode::Light),
        1_000,
    );
    let mut app = CrewApp::default();
    app.apply_chat_action(ChatAction::PersistTheme, 0);
    assert_eq!(app.config.theme.as_deref(), Some("light"));
}

#[test]
fn font_action_runs_the_input_bar_font_path() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    app.apply_chat_action(ChatAction::Font("18".into()), 0);
    assert_eq!(app.config.font_size, 18.0);
}

#[test]
fn font_action_echoes_the_status_into_the_pane() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    app.panes.push(crate::pane::Pane {
        glide: crate::glide::Glide::default(),
        content: crate::pane::PaneContent::Chat(crate::chat::ChatPane::new(plugin, "crew".into())),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    });
    app.apply_chat_action(ChatAction::Font("18".into()), 0);
    let crate::pane::PaneContent::Chat(c) = &app.panes[0].content else {
        panic!("chat pane replaced");
    };
    let last = c.messages.last().expect("a status note in the transcript");
    assert_eq!(last.sender, "agent smith");
    assert!(
        last.text.contains("font size 18"),
        "note should carry the /font status: {}",
        last.text
    );
}

#[test]
fn persist_theme_clears_stale_look_overrides() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Mode(crew_theme::RandomMode::Crt),
        1_000,
    );
    let mut app = CrewApp::default();
    app.config.crt = Some(false);
    app.config.glass = "off".to_string();
    app.apply_chat_action(ChatAction::PersistTheme, 0);
    assert_eq!(
        app.config.crt, None,
        "a composer theme switch clears the /crt pin like /theme does"
    );
    assert_eq!(app.config.glass, "medium");
}

#[test]
fn persist_theme_saves_a_fixed_theme_name() {
    let _g = crate::app::theme_test_guard();
    crew_theme::apply_selection(
        crew_theme::Selection::Fixed(crew_theme::ThemeId::PaperDark),
        1_000,
    );
    let mut app = CrewApp::default();
    app.apply_chat_action(ChatAction::PersistTheme, 0);
    assert_eq!(app.config.theme.as_deref(), Some("paper-dark"));
}
