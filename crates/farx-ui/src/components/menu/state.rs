use crossterm::event::{KeyCode, KeyEvent};

use super::definitions::build_menus;
use super::{MenuAction, MenuColumn};

pub struct MenuState {
    pub active: bool,
    pub(super) active_menu: usize,
    pub(super) active_item: usize,
    pub(super) dropdown_open: bool,
    pub(super) menus: Vec<MenuColumn>,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            active: true,
            active_menu: 0,
            active_item: 0,
            dropdown_open: true,
            menus: build_menus(),
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> MenuAction {
        match key.code {
            KeyCode::Esc | KeyCode::F(9) => {
                self.active = false;
                MenuAction::Close
            }
            KeyCode::Left => {
                if self.active_menu > 0 {
                    self.active_menu -= 1;
                } else {
                    self.active_menu = self.menus.len() - 1;
                }
                self.active_item = 0;
                MenuAction::None
            }
            KeyCode::Right => {
                self.active_menu = (self.active_menu + 1) % self.menus.len();
                self.active_item = 0;
                MenuAction::None
            }
            KeyCode::Up => {
                let menu = &self.menus[self.active_menu];
                if self.active_item > 0 {
                    self.active_item -= 1;
                    // Skip separators
                    while self.active_item > 0
                        && menu.items[self.active_item].action == MenuAction::None
                    {
                        self.active_item -= 1;
                    }
                }
                MenuAction::None
            }
            KeyCode::Down => {
                let item_count = self.menus[self.active_menu].items.len();
                if self.active_item + 1 < item_count {
                    self.active_item += 1;
                    // Skip separators
                    while self.active_item + 1 < item_count
                        && self.menus[self.active_menu].items[self.active_item].action
                            == MenuAction::None
                    {
                        self.active_item += 1;
                    }
                }
                MenuAction::None
            }
            KeyCode::Enter => {
                let action = self.menus[self.active_menu].items[self.active_item]
                    .action
                    .clone();
                if action != MenuAction::None {
                    self.active = false;
                }
                action
            }
            _ => MenuAction::None,
        }
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}
