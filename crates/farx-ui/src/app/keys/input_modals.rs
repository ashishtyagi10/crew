//! Mid-priority overlay modals (group B): batch rename, chmod dialog,
//! bookmarks panel, AI bar, and the generic input dialog. These tend to
//! mutate persistent state when closed (rename files, set perms, edit
//! bookmark list, submit AI query, apply pending op).

use crossterm::event::KeyEvent;
use farx_core::Action;

use crate::components::ai_bar::AiBarAction;
use crate::components::batch_rename::BatchRenameAction;
use crate::components::bookmarks::{save_bookmarks, BookmarkAction};
use crate::components::chmod_dialog::ChmodAction;

use super::super::pending::UndoEntry;
use super::super::App;

impl App {
    pub(super) fn key_route_modals(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(ref mut br) = self.batch_rename {
            match br.handle_key_event(key) {
                BatchRenameAction::Close => self.batch_rename = None,
                BatchRenameAction::Apply(renames) => {
                    self.batch_rename = None;
                    let mut ok = 0;
                    let mut fail = 0;
                    for (old_path, new_name) in &renames {
                        if let Some(parent) = old_path.parent() {
                            let new_path = parent.join(new_name);
                            match farx_fs::rename_entry(old_path, &new_path) {
                                Ok(()) => {
                                    self.undo_stack.push(UndoEntry::Rename {
                                        old: old_path.clone(),
                                        new: new_path,
                                    });
                                    ok += 1;
                                }
                                Err(_) => fail += 1,
                            }
                        }
                    }
                    if fail == 0 {
                        self.feedback.success(format!("Renamed {} file(s)", ok));
                    } else {
                        self.feedback
                            .warning(format!("Renamed {}, failed {}", ok, fail));
                    }
                    self.active_tree().rebuild();
                }
                BatchRenameAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut chmod) = self.chmod_dialog {
            match chmod.handle_key_event(key) {
                ChmodAction::Cancel => self.chmod_dialog = None,
                ChmodAction::Apply(new_mode) => {
                    let path = chmod.file_path.clone();
                    self.chmod_dialog = None;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        match std::fs::set_permissions(
                            &path,
                            std::fs::Permissions::from_mode(new_mode),
                        ) {
                            Ok(()) => {
                                self.feedback
                                    .success(format!("Permissions set to {:04o}", new_mode));
                                self.active_tree().rebuild();
                            }
                            Err(e) => {
                                self.feedback
                                    .error(format!("Failed to set permissions: {}", e));
                            }
                        }
                    }
                    let _ = path;
                }
                ChmodAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut bm_panel) = self.bookmarks_panel {
            match bm_panel.handle_key_event(key) {
                BookmarkAction::Close => self.bookmarks_panel = None,
                BookmarkAction::GoTo(path) => {
                    self.bookmarks_panel = None;
                    if path.is_dir() {
                        self.navigate_to(path);
                    } else {
                        self.feedback
                            .error("Bookmark path no longer exists".to_string());
                    }
                }
                BookmarkAction::Delete(idx) => {
                    if idx < self.bookmarks.len() {
                        self.bookmarks.remove(idx);
                        save_bookmarks(&self.bookmarks);
                    }
                }
                BookmarkAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut ai_bar) = self.ai_bar {
            match ai_bar.handle_key_event(key) {
                AiBarAction::Close => self.ai_bar = None,
                AiBarAction::Submit(query) => self.submit_ai_query(query),
                AiBarAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut dialog) = self.dialog {
            dialog.handle_key_event(key);
            if dialog.is_resolved() {
                let result = dialog.result.clone();
                let pending = self.pending_op.take();
                self.dialog = None;
                self.handle_dialog_result(result, pending);
            }
            return Some(Action::Noop);
        }

        None
    }
}
