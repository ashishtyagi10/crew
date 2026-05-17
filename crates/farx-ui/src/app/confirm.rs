//! Execute confirmed bulk file operations (copy / move / delete). Multi-file
//! copy and move spawn a worker thread and surface progress via a modal.

use crate::components::feedback::ConfirmAction;
use crate::components::progress::ProgressState;

use super::pending::UndoEntry;
use super::App;

impl App {
    /// Execute a confirmed file operation.
    pub(super) fn execute_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::Copy { sources, dest } => {
                if sources.len() >= 2 {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let srcs = sources.clone();
                    let dst = dest.clone();
                    std::thread::spawn(move || {
                        farx_fs::copy_entries_with_progress(srcs, dst, tx);
                    });
                    self.progress = Some(ProgressState::new("Copying", rx));
                } else {
                    let mut ok = 0;
                    let mut fail = 0;
                    for source in &sources {
                        match farx_fs::copy_entry(source, &dest) {
                            Ok(()) => ok += 1,
                            Err(_) => fail += 1,
                        }
                    }
                    if fail == 0 {
                        self.feedback.success(format!("Copied {} file(s)", ok));
                    } else {
                        self.feedback
                            .warning(format!("Copied {}, failed {}", ok, fail));
                    }
                }
            }
            ConfirmAction::Move { sources, dest } => {
                if sources.len() >= 2 {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let srcs = sources.clone();
                    let dst = dest.clone();
                    std::thread::spawn(move || {
                        farx_fs::move_entries_with_progress(srcs, dst, tx);
                    });
                    self.progress = Some(ProgressState::new("Moving", rx));
                    self.undo_stack.push(UndoEntry::Move { sources, dest });
                } else {
                    let mut ok = 0;
                    let mut fail = 0;
                    let moved_sources = sources.clone();
                    for source in &sources {
                        match farx_fs::move_entry(source, &dest) {
                            Ok(()) => ok += 1,
                            Err(_) => fail += 1,
                        }
                    }
                    if ok > 0 {
                        self.undo_stack.push(UndoEntry::Move {
                            sources: moved_sources,
                            dest: dest.clone(),
                        });
                    }
                    if fail == 0 {
                        self.feedback.success(format!("Moved {} file(s)", ok));
                    } else {
                        self.feedback
                            .warning(format!("Moved {}, failed {}", ok, fail));
                    }
                }
            }
            ConfirmAction::Delete { targets } => {
                let use_trash = self.config.general.use_trash;
                let mut ok = 0;
                let mut fail = 0;
                for target in &targets {
                    match farx_fs::delete_entry(target, use_trash) {
                        Ok(()) => ok += 1,
                        Err(_) => fail += 1,
                    }
                }
                if ok > 0 && use_trash {
                    self.undo_stack.push(UndoEntry::Delete {
                        paths: targets.clone(),
                    });
                }
                let verb = if use_trash { "Trashed" } else { "Deleted" };
                if fail == 0 {
                    self.feedback.success(format!("{} {} file(s)", verb, ok));
                } else {
                    self.feedback
                        .warning(format!("{} {}, failed {}", verb, ok, fail));
                }
            }
        }
        self.left_tree.rebuild();
        self.right_tree.rebuild();
    }
}
