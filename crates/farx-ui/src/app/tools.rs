//! Two read-only "inspect" tools: a text treemap of disk usage and a
//! per-cursor / per-selection size calculator. Plus the quick-action
//! palette dispatcher.

use farx_core::Action;

use super::helpers::{dir_size_recursive, format_size_human};
use super::App;

impl App {
    /// Show a text-based treemap of disk usage for the current directory.
    pub(super) fn show_treemap(&mut self) {
        let root = self.active_tree_ref().root.clone();
        let mut entries: Vec<(String, u64)> = Vec::new();

        if let Ok(rd) = std::fs::read_dir(&root) {
            for entry in rd.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let path = entry.path();
                let size = if path.is_dir() {
                    dir_size_recursive(&path)
                } else {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                };
                if size > 0 {
                    entries.push((name, size));
                }
            }
        }

        if entries.is_empty() {
            self.feedback.info("Directory is empty".to_string());
            return;
        }

        entries.sort_by(|a, b| b.1.cmp(&a.1));
        let total: u64 = entries.iter().map(|(_, s)| *s).sum();

        let mut lines = Vec::new();
        lines.push(format!("Disk Usage: {} total", format_size_human(total)));
        lines.push(String::new());

        let bar_width = 40usize;
        for (name, size) in entries.iter().take(30) {
            let pct = *size as f64 / total as f64;
            let filled = (pct * bar_width as f64).round() as usize;
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));
            lines.push(format!(
                "{} {:>5.1}% {:>9}  {}",
                bar,
                pct * 100.0,
                format_size_human(*size),
                name
            ));
        }

        if entries.len() > 30 {
            lines.push(format!("  ... and {} more entries", entries.len() - 30));
        }

        self.feedback
            .show_output("Disk Usage Treemap", lines.join("\n"));
    }

    /// Calculate the size of the directory (or selected items) under the cursor.
    pub(super) fn calculate_dir_size(&mut self) {
        let tree = self.active_tree_ref();

        if !tree.selected.is_empty() {
            let mut total: u64 = 0;
            let mut count = 0usize;
            let mut dir_count = 0usize;
            for &idx in &tree.selected {
                if let Some(node) = tree.visible_nodes.get(idx) {
                    count += 1;
                    if node.entry.is_dir {
                        dir_count += 1;
                        total += dir_size_recursive(&node.entry.path);
                    } else {
                        total += node.entry.size;
                    }
                }
            }
            let desc = if dir_count > 0 {
                format!(
                    "{} items ({} dirs): {}",
                    count,
                    dir_count,
                    format_size_human(total)
                )
            } else {
                format!("{} files: {}", count, format_size_human(total))
            };
            self.feedback.info(desc);
            return;
        }

        if let Some(node) = tree.current_node() {
            let path = node.entry.path.clone();
            let name = node.entry.name.clone();
            if path.is_dir() {
                let size = dir_size_recursive(&path);
                self.feedback
                    .info(format!("{}: {}", name, format_size_human(size)));
            } else {
                self.feedback
                    .info(format!("{}: {}", name, format_size_human(node.entry.size)));
            }
        }
    }

    /// Handle a quick action command (may be a special builtin or shell command).
    pub(super) fn handle_quick_action(&mut self, cmd: &str) {
        match cmd {
            "__open__" => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    let path = node.entry.path.clone();
                    let name = node.entry.name.clone();
                    match open::that(&path) {
                        Ok(()) => self.feedback.info(format!("Opened: {}", name)),
                        Err(e) => self.feedback.error(format!("Open: {}", e)),
                    }
                }
            }
            "__edit__" => self.dispatch(Action::EditFile),
            "__view__" => self.dispatch(Action::ViewFile),
            "__clipboard__" => self.dispatch(Action::CopyPathToClipboard),
            "__extract__" => self.dispatch(Action::ExtractArchive),
            "__view_archive__" => self.dispatch(Action::ViewArchive),
            "__terminal__" => {
                let dir = self.active_tree_ref().root.to_string_lossy().to_string();
                let cmd = if cfg!(target_os = "macos") {
                    format!("open -a Terminal {}", dir)
                } else {
                    format!("xterm -e 'cd {} && $SHELL' &", dir)
                };
                let _ = std::process::Command::new("sh").args(["-c", &cmd]).spawn();
            }
            shell_cmd => {
                self.command_line.input = shell_cmd.to_string();
                self.smart_execute_command();
            }
        }
    }
}
