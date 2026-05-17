//! Mid-priority overlay modals (group A): menu, search, fuzzy finder,
//! AI tools panel, and quick actions palette.

use crossterm::event::KeyEvent;
use farx_core::Action;

use crate::components::ai_panel::AiPanelAction;
use crate::components::fuzzy_finder::FuzzyAction;
use crate::components::quick_actions::QuickActionResult;
use crate::components::search::SearchAction;

use super::super::App;

impl App {
    pub(super) fn key_route_overlays(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(ref mut menu) = self.menu {
            let action = menu.handle_key_event(key);
            if !menu.active {
                self.menu = None;
            }
            self.handle_menu_action(action);
            return Some(Action::Noop);
        }

        if let Some(ref mut search) = self.search {
            let action = search.handle_key_event(key);
            if !search.active {
                self.search = None;
            }
            if let SearchAction::GoTo(path) = action {
                let show_hidden = self.config.general.show_hidden_files;
                let panel = self.active_panel_mut();
                panel.current_dir = path;
                panel.cursor = 0;
                panel.scroll_offset = 0;
                panel.selected.clear();
                Self::refresh_panel(panel, show_hidden);
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut ff) = self.fuzzy_finder {
            match ff.handle_key_event(key) {
                FuzzyAction::Close => self.fuzzy_finder = None,
                FuzzyAction::GoTo(path) => {
                    self.fuzzy_finder = None;
                    if path.is_dir() {
                        self.navigate_to(path);
                    } else if let Some(parent) = path.parent() {
                        self.navigate_to(parent.to_path_buf());
                    }
                }
                FuzzyAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut ai_panel) = self.ai_panel {
            match ai_panel.handle_key_event(key) {
                AiPanelAction::Close => self.ai_panel = None,
                AiPanelAction::Launch(tool) => {
                    self.ai_panel = None;
                    let (cmd, args) = tool.command();
                    self.spawn_embedded_terminal(cmd, args);
                }
                AiPanelAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut qa) = self.quick_actions {
            match qa.handle_key_event(key) {
                QuickActionResult::Close => self.quick_actions = None,
                QuickActionResult::Execute(cmd) => {
                    self.quick_actions = None;
                    self.handle_quick_action(&cmd);
                }
                QuickActionResult::None => {}
            }
            return Some(Action::Noop);
        }

        None
    }
}
