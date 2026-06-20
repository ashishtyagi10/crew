//! Bulk-mutation actions: batch rename, undo of recent ops, touch, and the
//! cross-panel directory comparison.

use std::path::PathBuf;

use farx_core::Action;

use crate::components::batch_rename::BatchRenameState;

use super::super::pending::UndoEntry;
use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_bulk_ops(&mut self, action: &Action) -> bool {
        match action {
            Action::BatchRename => self.start_batch_rename(),
            Action::Undo => self.pop_undo(),
            Action::TouchFile => self.touch_files(),
            Action::CompareDirectories => self.compare_directories(),
            _ => return false,
        }
        true
    }

    fn start_batch_rename(&mut self) {
        let tree = self.active_tree_ref();
        let files: Vec<(PathBuf, String)> = if !tree.selected.is_empty() {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .filter(|n| !n.entry.is_dir)
                .map(|n| (n.entry.path.clone(), n.entry.name.clone()))
                .collect()
        } else {
            tree.visible_nodes
                .iter()
                .filter(|n| !n.entry.is_dir && n.depth == 0)
                .map(|n| (n.entry.path.clone(), n.entry.name.clone()))
                .collect()
        };
        if files.is_empty() {
            self.feedback.error("No files to rename".to_string());
        } else {
            self.batch_rename = Some(BatchRenameState::new(files));
        }
    }

    fn pop_undo(&mut self) {
        let Some(entry) = self.undo_stack.pop() else {
            self.feedback.info("Nothing to undo".to_string());
            return;
        };
        match entry {
            UndoEntry::Delete { paths } => {
                let names: Vec<String> = paths
                    .iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .collect();
                self.feedback.info(format!(
                    "Undo: check system trash for: {}",
                    names.join(", ")
                ));
            }
            UndoEntry::Move { sources, dest } => {
                let mut ok = 0;
                for source in &sources {
                    let name = source
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let moved_to = dest.join(&name);
                    let original_dir = source.parent().unwrap_or(std::path::Path::new("/"));
                    if farx_fs::move_entry(&moved_to, original_dir).is_ok() {
                        ok += 1;
                    }
                }
                self.feedback
                    .success(format!("Undo: moved {} file(s) back", ok));
                self.left_tree.rebuild();
                self.right_tree.rebuild();
            }
            UndoEntry::Rename { old, new } => match farx_fs::rename_entry(&new, &old) {
                Ok(()) => {
                    self.feedback.success("Undo: rename reversed".to_string());
                    self.active_tree().rebuild();
                }
                Err(e) => self.feedback.error(format!("Undo rename: {}", e)),
            },
        }
    }

    fn touch_files(&mut self) {
        let paths = self.collect_selected_paths();
        if paths.is_empty() {
            return;
        }
        let now = std::time::SystemTime::now();
        let mut ok = 0;
        let mut fail = 0;
        for path in &paths {
            if path.exists() {
                match std::fs::File::options().write(true).open(path) {
                    Ok(f) => match f.set_modified(now) {
                        Ok(()) => ok += 1,
                        Err(_) => fail += 1,
                    },
                    Err(_) => fail += 1,
                }
            } else {
                match std::fs::File::create(path) {
                    Ok(_) => ok += 1,
                    Err(_) => fail += 1,
                }
            }
        }
        self.active_tree().rebuild();
        if fail == 0 {
            self.feedback.success(format!("Touched {} file(s)", ok));
        } else {
            self.feedback
                .warning(format!("Touched {}, failed {}", ok, fail));
        }
    }
}
