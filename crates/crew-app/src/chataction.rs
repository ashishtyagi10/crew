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
                // Same contract as `set_theme_cmd`: switching themes clears the
                // stale `/crt` pin and a glass `off` so the theme's look shows.
                if self.config.reset_look_overrides() {
                    self.apply_glass();
                }
                self.config.save();
            }
            ChatAction::FindJump => {
                let Some(pane) = self.panes.get_mut(focused) else {
                    return;
                };
                let (cols, rows) = (pane.grid.cols, pane.grid.rows);
                if let crate::pane::PaneContent::Chat(c) = &mut pane.content {
                    crate::chatfind::jump(c, cols, rows);
                }
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
#[path = "chataction_tests.rs"]
mod tests;
