//! Dialog result handling: turn a closed input-dialog's value into the
//! actual filesystem operation (mkdir / rename / create / symlink / copy
//! same-dir) or non-fs side effect (mask selection, goto-dir).

use std::path::PathBuf;

use crate::components::dialog::DialogResult;

use super::pending::{PendingOperation, UndoEntry};
use super::App;

impl App {
    /// Process the result of a closed dialog and execute the corresponding file operation.
    pub(super) fn handle_dialog_result(
        &mut self,
        result: DialogResult,
        pending: Option<PendingOperation>,
    ) {
        match result {
            DialogResult::Confirm(input_value) => {
                if let Some(op) = pending {
                    self.execute_pending_operation(op, input_value);
                }
            }
            DialogResult::Cancel | DialogResult::Pending => {}
        }
    }

    /// Execute the file operation associated with a confirmed input dialog.
    fn execute_pending_operation(&mut self, op: PendingOperation, input_value: Option<String>) {
        match &op {
            PendingOperation::SelectByMask | PendingOperation::DeselectByMask => {
                let selecting = matches!(op, PendingOperation::SelectByMask);
                if let Some(pattern) = input_value {
                    let pattern = pattern.trim();
                    if pattern.is_empty() {
                        return;
                    }
                    self.apply_mask_selection(pattern, selecting);
                }
                return;
            }
            PendingOperation::GotoDirectory => {
                if let Some(path_str) = input_value {
                    let path_str = path_str.trim();
                    if path_str.is_empty() {
                        return;
                    }
                    let path = if path_str.starts_with('~') {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(path_str.trim_start_matches("~/"))
                    } else {
                        PathBuf::from(path_str)
                    };
                    if path.is_dir() {
                        self.navigate_to(path);
                    } else {
                        self.feedback
                            .error(format!("Not a directory: {}", path_str));
                    }
                }
                return;
            }
            _ => {}
        }

        let result = match op {
            PendingOperation::MkDir { parent } => match input_value {
                Some(name) => {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    farx_fs::create_directory(&parent.join(name))
                }
                None => return,
            },
            PendingOperation::Rename { original } => match input_value {
                Some(new_name) => {
                    let new_name = new_name.trim();
                    if new_name.is_empty() {
                        return;
                    }
                    if let Some(parent) = original.parent() {
                        let new_path = parent.join(new_name);
                        let old_clone = original.clone();
                        let new_clone = new_path.clone();
                        let result = farx_fs::rename_entry(&original, &new_path);
                        if result.is_ok() {
                            self.undo_stack.push(UndoEntry::Rename {
                                old: old_clone,
                                new: new_clone,
                            });
                        }
                        result
                    } else {
                        return;
                    }
                }
                None => return,
            },
            PendingOperation::CreateFile { parent } => match input_value {
                Some(name) => {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let file_path = parent.join(name);
                    if let Some(file_parent) = file_path.parent() {
                        if !file_parent.exists() {
                            if let Err(e) = std::fs::create_dir_all(file_parent) {
                                self.show_error("Create File", &format!("{e}"));
                                return;
                            }
                        }
                    }
                    std::fs::File::create(&file_path)
                        .map(|_| ())
                        .map_err(anyhow::Error::from)
                }
                None => return,
            },
            PendingOperation::CopySameDir { source } => match input_value {
                Some(name) => {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let dest = self.active_tree_ref().root.clone().join(name);
                    std::fs::copy(&source, &dest)
                        .map(|_| ())
                        .map_err(anyhow::Error::from)
                }
                None => return,
            },
            PendingOperation::CreateSymlink { target } => match input_value {
                Some(name) => {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let link_path = self.active_tree_ref().root.clone().join(name);
                    farx_fs::create_symlink(&target, &link_path)
                }
                None => return,
            },
            PendingOperation::SelectByMask
            | PendingOperation::DeselectByMask
            | PendingOperation::GotoDirectory => return,
        };

        self.left_tree.rebuild();
        self.right_tree.rebuild();

        match result {
            Ok(()) => self.feedback.success("Done"),
            Err(e) => self.feedback.error(format!("{e}")),
        }
    }

    pub(super) fn show_error(&mut self, title: &str, message: &str) {
        self.feedback.error(format!("{}: {}", title, message));
    }
}
