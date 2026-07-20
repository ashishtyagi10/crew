//! App-side execution of the `ChatAction`s a chat pane's key handler returns:
//! close the pane, persist a `/theme` switch made from the pane's composer
//! (the pane applies it live but cannot reach the config), and run `/font`
//! through the same path as the input bar.
use crate::app::CrewApp;
use crate::chatkeys::ChatAction;

impl CrewApp {
    /// Execute a `ChatAction` from the pane at `focused`.
    pub(crate) fn apply_chat_action(&mut self, action: ChatAction, focused: usize) {
        match action {
            ChatAction::Close => {
                self.close_pane(focused);
            }
            ChatAction::PersistTheme => {
                self.config.theme = Some(crew_theme::selection_label().to_string());
                crate::palette::set_accent(self.config.accent_rgb());
                self.config.save();
            }
            ChatAction::Font(arg) => {
                self.set_font_cmd(&arg);
                // Echo the outcome into the pane's transcript too — the
                // composer submission vanished, so the status line alone is
                // easy to miss.
                if let Some((note, _)) = self.status.clone() {
                    if let Some(crate::pane::PaneContent::Chat(c)) =
                        self.panes.get_mut(focused).map(|p| &mut p.content)
                    {
                        c.push_note(note);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
            content: crate::pane::PaneContent::Chat(crate::chat::ChatPane::new(
                plugin,
                "crew".into(),
            )),
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
        });
        app.apply_chat_action(ChatAction::Font("18".into()), 0);
        let crate::pane::PaneContent::Chat(c) = &app.panes[0].content else {
            panic!("chat pane replaced");
        };
        let last = c.messages.last().expect("a status note in the transcript");
        assert_eq!(last.sender, "crew");
        assert!(
            last.text.contains("font size 18"),
            "note should carry the /font status: {}",
            last.text
        );
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
}
