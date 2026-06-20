//! Chrome rendering and chrome-driven dispatch: the bottom status bar
//! and the menu-action router.

use farx_core::{Action, SortField};

use crate::components::menu::MenuAction;

use super::App;

impl App {
    /// Status text for the command-box footer: "no agents" when idle,
    /// otherwise "<focused title> · N agents".
    pub(super) fn agent_status_text(&self) -> String {
        if self.grid.is_empty() {
            return "no agents".to_string();
        }
        let count = self.grid.len();
        let title = self
            .focused_terminal
            .and_then(|id| self.terminal_by_id(id))
            .map(|t| t.title.clone())
            .unwrap_or_else(|| "—".to_string());
        let count_label = if count == 1 {
            "1 agent".to_string()
        } else {
            format!("{} agents", count)
        };
        format!("{} · {}", title, count_label)
    }

    /// Translate a closed menu's selection into the equivalent dispatch.
    pub(super) fn handle_menu_action(&mut self, action: MenuAction) {
        let show_hidden = self.config.general.show_hidden_files;
        match action {
            MenuAction::SortByName => self.toggle_sort(SortField::Name),
            MenuAction::SortByExtension => self.toggle_sort(SortField::Extension),
            MenuAction::SortBySize => self.toggle_sort(SortField::Size),
            MenuAction::SortByDate => self.toggle_sort(SortField::Modified),
            MenuAction::ToggleHidden => {
                self.config.general.show_hidden_files = !self.config.general.show_hidden_files;
                self.refresh_both_panels();
            }
            MenuAction::Refresh => {
                Self::refresh_panel(self.active_panel_mut(), show_hidden);
            }
            MenuAction::ViewFile => self.dispatch(Action::ViewFile),
            MenuAction::EditFile => self.dispatch(Action::EditFile),
            MenuAction::CopyFile => self.dispatch(Action::CopyDialog),
            MenuAction::MoveFile => self.dispatch(Action::MoveDialog),
            MenuAction::DeleteFile => self.dispatch(Action::DeleteDialog),
            MenuAction::MkDir => self.dispatch(Action::MkDirDialog),
            MenuAction::FindFiles => self.dispatch(Action::ShowSearchDialog),
            MenuAction::ShowAiBar => self.dispatch(Action::ShowAiBar),
            MenuAction::ShowAiPanel => self.dispatch(Action::ShowAiPanel),
            MenuAction::SwapPanels => self.dispatch(Action::SwapPanels),
            MenuAction::None | MenuAction::Close => {}
        }
    }
}
