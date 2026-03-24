use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    None,
    Close,
    // Panel actions
    SortByName,
    SortByExtension,
    SortBySize,
    SortByDate,
    ToggleHidden,
    Refresh,
    // File actions
    ViewFile,
    EditFile,
    CopyFile,
    MoveFile,
    DeleteFile,
    MkDir,
    // Commands
    FindFiles,
    ShowAiBar,
    SwapPanels,
    // Options
    ToggleFnBar,
}

struct MenuItem {
    label: &'static str,
    action: MenuAction,
    hotkey: &'static str,
}

struct MenuColumn {
    title: &'static str,
    items: Vec<MenuItem>,
}

pub struct MenuState {
    pub active: bool,
    active_menu: usize,
    active_item: usize,
    dropdown_open: bool,
    menus: Vec<MenuColumn>,
}

impl MenuState {
    pub fn new() -> Self {
        let menus = vec![
            MenuColumn {
                title: " Left ",
                items: vec![
                    MenuItem {
                        label: "Sort by Name",
                        action: MenuAction::SortByName,
                        hotkey: "Ctrl+F3",
                    },
                    MenuItem {
                        label: "Sort by Extension",
                        action: MenuAction::SortByExtension,
                        hotkey: "Ctrl+F4",
                    },
                    MenuItem {
                        label: "Sort by Size",
                        action: MenuAction::SortBySize,
                        hotkey: "Ctrl+F5",
                    },
                    MenuItem {
                        label: "Sort by Date",
                        action: MenuAction::SortByDate,
                        hotkey: "Ctrl+F6",
                    },
                    MenuItem {
                        label: "─────────────",
                        action: MenuAction::None,
                        hotkey: "",
                    },
                    MenuItem {
                        label: "Toggle Hidden",
                        action: MenuAction::ToggleHidden,
                        hotkey: "Ctrl+H",
                    },
                    MenuItem {
                        label: "Refresh",
                        action: MenuAction::Refresh,
                        hotkey: "Ctrl+R",
                    },
                ],
            },
            MenuColumn {
                title: " Files ",
                items: vec![
                    MenuItem {
                        label: "View",
                        action: MenuAction::ViewFile,
                        hotkey: "F3",
                    },
                    MenuItem {
                        label: "Edit",
                        action: MenuAction::EditFile,
                        hotkey: "F4",
                    },
                    MenuItem {
                        label: "Copy",
                        action: MenuAction::CopyFile,
                        hotkey: "F5",
                    },
                    MenuItem {
                        label: "Move/Rename",
                        action: MenuAction::MoveFile,
                        hotkey: "F6",
                    },
                    MenuItem {
                        label: "Make Directory",
                        action: MenuAction::MkDir,
                        hotkey: "F7",
                    },
                    MenuItem {
                        label: "Delete",
                        action: MenuAction::DeleteFile,
                        hotkey: "F8",
                    },
                ],
            },
            MenuColumn {
                title: " Commands ",
                items: vec![
                    MenuItem {
                        label: "Find Files",
                        action: MenuAction::FindFiles,
                        hotkey: "Alt+F7",
                    },
                    MenuItem {
                        label: "AI Assistant",
                        action: MenuAction::ShowAiBar,
                        hotkey: "Ctrl+Space",
                    },
                    MenuItem {
                        label: "Swap Panels",
                        action: MenuAction::SwapPanels,
                        hotkey: "",
                    },
                ],
            },
            MenuColumn {
                title: " Options ",
                items: vec![MenuItem {
                    label: "Toggle Fn Bar",
                    action: MenuAction::ToggleFnBar,
                    hotkey: "",
                }],
            },
        ];

        Self {
            active: true,
            active_menu: 0,
            active_item: 0,
            dropdown_open: true,
            menus,
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

pub fn render_menu(frame: &mut Frame, state: &MenuState, _theme: &Theme) {
    let area = frame.area();

    // Menu bar at top (1 line)
    let bar_area = Rect::new(area.x, area.y, area.width, 1);
    let bar_bg = Style::default().fg(Color::Black).bg(Color::Cyan);

    // Build menu bar line
    let mut spans = Vec::new();
    let mut x_positions: Vec<u16> = Vec::new();
    let mut x = 0u16;

    for (i, menu) in state.menus.iter().enumerate() {
        x_positions.push(x);
        let style = if i == state.active_menu {
            Style::default().fg(Color::White).bg(Color::Black)
        } else {
            bar_bg
        };
        let label = menu.title;
        spans.push(Span::styled(label, style));
        x += label.len() as u16;
    }
    // Fill rest of bar
    let remaining = area.width.saturating_sub(x) as usize;
    spans.push(Span::styled(" ".repeat(remaining), bar_bg));

    frame.render_widget(Paragraph::new(Line::from(spans)), bar_area);

    // Draw dropdown for active menu
    if state.dropdown_open && state.active_menu < state.menus.len() {
        let menu = &state.menus[state.active_menu];
        let dropdown_x = x_positions[state.active_menu];

        // Calculate dropdown width
        let max_label = menu
            .items
            .iter()
            .map(|i| i.label.len() + i.hotkey.len() + 4)
            .max()
            .unwrap_or(20);
        let dropdown_width = (max_label as u16 + 2).min(area.width - dropdown_x);
        let dropdown_height = menu.items.len() as u16 + 2; // +2 for borders

        let dropdown_area = Rect::new(
            dropdown_x,
            1, // right below the menu bar
            dropdown_width,
            dropdown_height.min(area.height.saturating_sub(1)),
        );

        frame.render_widget(Clear, dropdown_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow).bg(Color::Indexed(236)))
            .style(Style::default().bg(Color::Indexed(236)));

        let inner = block.inner(dropdown_area);
        frame.render_widget(block, dropdown_area);

        for (i, item) in menu.items.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let is_separator = item.action == MenuAction::None;
            let is_selected = i == state.active_item;

            let item_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);

            if is_separator {
                let sep = "\u{2500}".repeat(inner.width as usize);
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        sep,
                        Style::default()
                            .fg(Color::DarkGray)
                            .bg(Color::Indexed(236)),
                    )),
                    item_area,
                );
            } else {
                let style = if is_selected {
                    Style::default().fg(Color::White).bg(Color::Indexed(24))
                } else {
                    Style::default().fg(Color::White).bg(Color::Indexed(236))
                };
                let hotkey_style = if is_selected {
                    Style::default().fg(Color::Yellow).bg(Color::Indexed(24))
                } else {
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(Color::Indexed(236))
                };

                let padding = inner.width as usize
                    - item.label.len().min(inner.width as usize)
                    - item.hotkey.len().min(inner.width as usize);
                let line = Line::from(vec![
                    Span::styled(item.label, style),
                    Span::styled(" ".repeat(padding.max(1)), style),
                    Span::styled(item.hotkey, hotkey_style),
                ]);
                frame.render_widget(Paragraph::new(line), item_area);
            }
        }
    }
}
