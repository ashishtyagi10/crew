//! Chrome rendering and chrome-driven dispatch: the bottom status bar,
//! function-key bar click handling, and the menu-action router.

use farx_core::{Action, SortField};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::components::menu::MenuAction;

use super::App;

impl App {
    /// Paint the one-row status bar at the bottom of the panel area.
    pub(super) fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let bg = ratatui::style::Color::Indexed(235);
        let label_style = ratatui::style::Style::default()
            .fg(ratatui::style::Color::Rgb(140, 140, 150))
            .bg(bg);
        let value_style = ratatui::style::Style::default()
            .fg(ratatui::style::Color::White)
            .bg(bg);
        let sep_style = ratatui::style::Style::default()
            .fg(ratatui::style::Color::Rgb(60, 60, 65))
            .bg(bg);

        let mut spans: Vec<Span<'_>> = Vec::new();
        spans.push(Span::styled(" ", label_style));

        if self.grid.is_empty() {
            spans.push(Span::styled("no agents", label_style));
        } else {
            let agent_count = self.grid.len();
            let title = self
                .focused_terminal
                .and_then(|id| self.terminal_by_id(id))
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "—".to_string());
            spans.push(Span::styled(title, value_style));
            spans.push(Span::styled(" · ", sep_style));
            let count_label = if agent_count == 1 {
                "1 agent".to_string()
            } else {
                format!("{} agents", agent_count)
            };
            spans.push(Span::styled(count_label, label_style));
        }

        let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let remaining = (area.width as usize).saturating_sub(used);
        if remaining > 0 {
            spans.push(Span::styled(" ".repeat(remaining), label_style));
        }

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Dispatch the action matching the clicked F-key slot in the bottom bar.
    pub(super) fn handle_fn_bar_click(&mut self, mx: u16, fn_rect: Rect) {
        let total_width = fn_rect.width as usize;
        let item_count = 10usize; // F1..F10
        let slot_width = if item_count > 0 {
            total_width / item_count
        } else {
            return;
        };
        let click_offset = (mx - fn_rect.x) as usize;
        let slot_index = click_offset / slot_width;
        let action = match slot_index {
            0 => Action::ShowHelp,
            1 => Action::OpenSystemApp,
            2 => Action::EditFile,
            3 => Action::SwitchPanel,
            4 => Action::CopyDialog,
            5 => Action::MoveDialog,
            6 => Action::MkDirDialog,
            7 => Action::DeleteDialog,
            8 => Action::ShowMenu,
            9 => Action::Quit,
            _ => return,
        };
        self.dispatch(action);
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
            MenuAction::ToggleFnBar => {
                self.config.ui.show_fn_bar = !self.config.ui.show_fn_bar;
            }
            MenuAction::None | MenuAction::Close => {}
        }
    }
}
