//! Mouse event handling: scroll routing, left-click (breadcrumb,
//! panel entry, double-click), and right-click (toggle select).

mod hit_test;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use farx_core::{Action, PanelSide};

use super::App;

impl App {
    /// Top-level mouse dispatcher.
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        let (mx, my) = (mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                if let Some(ref mut editor) = self.editor {
                    let amount: i32 = if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                        -3
                    } else {
                        3
                    };
                    editor.scroll_offset = if amount < 0 {
                        editor.scroll_offset.saturating_sub((-amount) as usize)
                    } else {
                        (editor.scroll_offset + amount as usize)
                            .min(editor.lines.len().saturating_sub(1))
                    };
                    return;
                }
                if let Some(ref mut viewer) = self.viewer {
                    viewer.handle_mouse_event(mouse);
                    return;
                }
                if let Some(side) = self.panel_side_at(mx, my) {
                    let tree = match side {
                        PanelSide::Left => &mut self.left_tree,
                        PanelSide::Right => &mut self.right_tree,
                    };
                    match mouse.kind {
                        MouseEventKind::ScrollUp => tree.move_cursor(-3),
                        MouseEventKind::ScrollDown => tree.move_cursor(3),
                        _ => {}
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.editor.is_some() || self.viewer.is_some() || self.help.is_some() {
                    return;
                }
                if self.dialog.is_some()
                    || self.menu.is_some()
                    || self.search.is_some()
                    || self.ai_bar.is_some()
                    || self.bookmarks_panel.is_some()
                    || self.fuzzy_finder.is_some()
                    || self.quick_actions.is_some()
                    || self.batch_rename.is_some()
                    || self.chmod_dialog.is_some()
                {
                    return;
                }

                if self.try_focus_terminal_at(mx, my) {
                    return;
                }

                if let Some((side, path)) = self.breadcrumb_hit(mx, my) {
                    self.active_panel = side;
                    if path.is_dir() {
                        let show_hidden = self.config.general.show_hidden_files;
                        let tree = match side {
                            PanelSide::Left => &mut self.left_tree,
                            PanelSide::Right => &mut self.right_tree,
                        };
                        tree.set_root(path);
                        tree.show_hidden = show_hidden;
                        tree.rebuild();
                    }
                    return;
                }

                if let Some((side, row_in_list)) = self.panel_row_at(mx, my) {
                    if self.active_panel != side {
                        self.active_panel = side;
                    }

                    if let Some(row) = row_in_list {
                        let tree = match side {
                            PanelSide::Left => &mut self.left_tree,
                            PanelSide::Right => &mut self.right_tree,
                        };
                        let target = tree.scroll_offset + row;
                        if target < tree.visible_nodes.len() {
                            tree.cursor = target;
                        }
                    }

                    let is_double = if let Some((lx, ly, lt)) = self.last_click {
                        lx == mx && ly == my && self.tick_count.saturating_sub(lt) < 8
                    } else {
                        false
                    };

                    if is_double {
                        self.last_click = None;
                        self.dispatch(Action::EnterDirectory);
                    } else {
                        self.last_click = Some((mx, my, self.tick_count));
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if self.editor.is_some() || self.viewer.is_some() || self.help.is_some() {
                    return;
                }
                if let Some((side, row_in_list)) = self.panel_row_at(mx, my) {
                    if self.active_panel != side {
                        self.active_panel = side;
                    }
                    if let Some(row) = row_in_list {
                        let tree = match side {
                            PanelSide::Left => &mut self.left_tree,
                            PanelSide::Right => &mut self.right_tree,
                        };
                        let target = tree.scroll_offset + row;
                        if target < tree.visible_nodes.len() {
                            tree.cursor = target;
                            if tree.selected.contains(&target) {
                                tree.selected.remove(&target);
                            } else {
                                tree.selected.insert(target);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
