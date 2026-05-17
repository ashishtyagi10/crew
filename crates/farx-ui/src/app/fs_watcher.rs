//! Filesystem-watching: notify-based change detection over the two panel
//! root directories, with a 2-tick debounce before the trees rebuild.

use super::App;

impl App {
    /// Set up the filesystem watcher to detect external changes.
    pub(super) fn setup_fs_watcher(&mut self) {
        use notify::{EventKind, RecursiveMode, Watcher};
        let (tx, rx) = std::sync::mpsc::channel();

        let handler_tx = tx.clone();
        let watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    match event.kind {
                        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                            let _ = handler_tx.send(());
                        }
                        _ => {}
                    }
                }
            });

        match watcher {
            Ok(mut w) => {
                let _ = w.watch(&self.left_tree.root, RecursiveMode::NonRecursive);
                let _ = w.watch(&self.right_tree.root, RecursiveMode::NonRecursive);
                self.fs_watcher = Some(w);
                self.fs_change_rx = Some(rx);
            }
            Err(_) => {
                // Watcher unavailable — silently skip
            }
        }
    }

    /// Re-watch directories when panels navigate.
    pub(super) fn update_fs_watcher(&mut self) {
        use notify::{RecursiveMode, Watcher};
        if let Some(ref mut w) = self.fs_watcher {
            let _ = w.unwatch(&self.left_tree.root);
            let _ = w.unwatch(&self.right_tree.root);
            let _ = w.watch(&self.left_tree.root, RecursiveMode::NonRecursive);
            let _ = w.watch(&self.right_tree.root, RecursiveMode::NonRecursive);
        }
    }

    /// Check for filesystem change notifications (debounced).
    pub(super) fn check_fs_changes(&mut self) {
        if let Some(ref rx) = self.fs_change_rx {
            let mut changed = false;
            while rx.try_recv().is_ok() {
                changed = true;
            }
            if changed {
                if self.tick_count - self.fs_change_tick >= 2 {
                    self.left_tree.rebuild();
                    self.right_tree.rebuild();
                }
                self.fs_change_tick = self.tick_count;
            }
        }
    }
}
