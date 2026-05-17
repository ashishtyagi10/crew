//! Tree-navigation routing: cursor motion, expand/collapse, enter-directory,
//! and the file-opening shortcuts (OpenSystemApp / ViewFile / EditFile).

use farx_core::Action;

use crate::components::editor::EditorState;
use crate::components::viewer::ViewerState;

use super::super::text_detect::is_text_file;
use super::super::App;

impl App {
    /// Handle tree-navigation actions. Returns `true` if the action was
    /// consumed; the caller should stop further dispatch.
    pub(in crate::app) fn dispatch_tree_nav(&mut self, action: &Action) -> bool {
        match action {
            Action::CursorUp => self.active_tree().move_cursor(-1),
            Action::CursorDown => self.active_tree().move_cursor(1),
            Action::CursorPageUp => self.active_tree().move_cursor(-20),
            Action::CursorPageDown => self.active_tree().move_cursor(20),
            Action::CursorHome => self.active_tree().move_cursor_to(0),
            Action::CursorEnd => {
                let last = self.active_tree().visible_nodes.len().saturating_sub(1);
                self.active_tree().move_cursor_to(last);
            }
            Action::TreeExpand => self.active_tree().expand(),
            Action::TreeCollapse => self.active_tree().collapse(),
            Action::EnterDirectory | Action::CommandLineEnterOrDir => {
                self.enter_directory_or_open(matches!(action, Action::CommandLineEnterOrDir));
            }
            Action::ParentDirectory => {
                let parent = self
                    .active_tree_ref()
                    .root
                    .parent()
                    .map(|p| p.to_path_buf());
                if let Some(parent_path) = parent {
                    self.navigate_to(parent_path);
                }
            }
            Action::OpenSystemApp => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    let path = node.entry.path.clone();
                    let name = node.entry.name.clone();
                    match open::that(&path) {
                        Ok(()) => self.feedback.info(format!("Opened: {}", name)),
                        Err(e) => self.feedback.error(format!("Open: {}", e)),
                    }
                }
            }
            Action::ViewFile => self.open_in_viewer(),
            Action::EditFile => self.open_in_editor(),
            _ => return false,
        }
        true
    }

    fn enter_directory_or_open(&mut self, from_command_line: bool) {
        if from_command_line && !self.command_line.input.is_empty() {
            self.smart_execute_command();
            return;
        }
        let node_info = self
            .active_tree_ref()
            .current_node()
            .map(|n| (n.entry.is_dir, n.entry.path.clone(), n.entry.name.clone()));
        let Some((is_dir, path, name)) = node_info else {
            return;
        };
        if is_dir {
            self.navigate_to(path);
        } else if farx_fs::is_archive(&path) {
            self.dispatch(Action::ViewArchive);
        } else if is_text_file(&path) {
            match EditorState::open(&path) {
                Ok(es) => self.editor = Some(es),
                Err(e) => self.show_error("Edit", &format!("{}", e)),
            }
        } else {
            match open::that(&path) {
                Ok(()) => self.feedback.info(format!("Opened: {}", name)),
                Err(e) => self.feedback.error(format!("Open: {}", e)),
            }
        }
    }

    fn open_in_viewer(&mut self) {
        if let Some(node) = self.active_tree().current_node() {
            if !node.entry.is_dir {
                let path = node.entry.path.clone();
                match ViewerState::open(&path) {
                    Ok(vs) => self.viewer = Some(vs),
                    Err(e) => self.show_error("View", &format!("{}", e)),
                }
            }
        }
    }

    fn open_in_editor(&mut self) {
        if let Some(node) = self.active_tree().current_node() {
            if !node.entry.is_dir {
                let path = node.entry.path.clone();
                match EditorState::open(&path) {
                    Ok(es) => self.editor = Some(es),
                    Err(e) => self.show_error("Edit", &format!("{}", e)),
                }
            }
        }
    }
}
