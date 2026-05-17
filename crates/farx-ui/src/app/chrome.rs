//! Chrome rendering and chrome-driven dispatch: the bottom status bar,
//! function-key bar click handling, and the menu-action router.

use farx_core::{Action, PanelSide, SortField};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::components::menu::MenuAction;

use super::helpers::{format_size_human, get_disk_space_cached};
use super::App;

impl App {
    /// Paint the one-row status bar at the bottom of the panel area.
    pub(super) fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let tree = self.active_tree_ref();
        let total_files = tree.visible_nodes.len();
        let selected_count = tree.selected.len();
        let selected_size: u64 = tree
            .selected
            .iter()
            .filter_map(|&i| tree.visible_nodes.get(i))
            .filter(|n| !n.entry.is_dir)
            .map(|n| n.entry.size)
            .sum();

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

        spans.push(Span::styled("Files: ", label_style));
        spans.push(Span::styled(format!("{}", total_files), value_style));

        if selected_count > 0 {
            spans.push(Span::styled(" │ ", sep_style));
            spans.push(Span::styled("Selected: ", label_style));
            spans.push(Span::styled(
                format!("{} ({})", selected_count, format_size_human(selected_size)),
                value_style,
            ));
        }

        spans.push(Span::styled(" │ ", sep_style));
        let (free, total) = get_disk_space_cached(&tree.root);
        if let (Some(free), Some(total)) = (free, total) {
            spans.push(Span::styled("Disk: ", label_style));
            spans.push(Span::styled(
                format!(
                    "{} free / {}",
                    format_size_human(free),
                    format_size_human(total)
                ),
                value_style,
            ));
        }

        let tab_group = match self.active_panel {
            PanelSide::Left => &self.left_tree,
            PanelSide::Right => &self.right_tree,
        };
        if tab_group.tab_count() > 1 {
            spans.push(Span::styled(" │ ", sep_style));
            spans.push(Span::styled("Tab: ", label_style));
            spans.push(Span::styled(
                format!("{}/{}", tab_group.active_tab() + 1, tab_group.tab_count()),
                value_style,
            ));
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
