mod accessors;
mod ai_glue;
mod chrome;
mod commands;
mod confirm;
mod dialogs;
mod dispatch;
mod fs_watcher;
mod globs;
mod helpers;
mod keys;
mod lifecycle;
mod mouse;
mod pending;
mod render;
mod selection_ops;
mod shell_commands;
mod slash;
mod state;
mod terminals;
mod text_detect;
mod tick;
mod tools;
mod update_flow;

use std::path::PathBuf;

use farx_core::{Action, PanelSide, SortField};

use crate::components::ai_bar::AiBarState;
use crate::components::ai_panel::AiPanelState;
use crate::components::batch_rename::BatchRenameState;
use crate::components::bookmarks::{save_bookmarks, Bookmark, BookmarkState};
use crate::components::chmod_dialog::ChmodDialogState;
use crate::components::dialog::DialogState;
use crate::components::diff_view::DiffViewState;
use crate::components::editor::EditorState;
use crate::components::feedback::ConfirmAction;
use crate::components::fuzzy_finder::FuzzyFinderState;
use crate::components::help::HelpState;
use crate::components::menu::MenuState;
use crate::components::quick_actions::QuickActionsState;
use crate::components::search::SearchState;

pub use self::state::App;

use self::helpers::format_size_human;
use self::pending::{PendingOperation, UndoEntry};

impl App {
    /// Execute an action, updating application state accordingly.
    pub fn dispatch(&mut self, action: Action) {
        if self.dispatch_tree_nav(&action) || self.dispatch_selection(&action) {
            return;
        }

        match action {
            Action::Quit => {
                self.running = false;
            }
            Action::SwitchPanel => {
                self.cycle_focus();
            }
            Action::FocusLeftPanel => {
                self.focused_terminal = None;
                self.active_panel = PanelSide::Left;
            }
            Action::FocusRightPanel => {
                self.focused_terminal = None;
                self.active_panel = PanelSide::Right;
            }
            Action::SwapPanels => {
                std::mem::swap(&mut self.left_panel, &mut self.right_panel);
                self.left_panel.side = PanelSide::Left;
                self.right_panel.side = PanelSide::Right;
                std::mem::swap(&mut self.left_tree, &mut self.right_tree);
                self.update_fs_watcher();
            }
            Action::GotoRoot => {
                let root = if cfg!(windows) {
                    PathBuf::from("C:\\")
                } else {
                    PathBuf::from("/")
                };
                self.navigate_to(root);
            }
            Action::ToggleHidden => {
                self.config.general.show_hidden_files = !self.config.general.show_hidden_files;
                let sh = self.config.general.show_hidden_files;
                self.left_tree.show_hidden = sh;
                self.left_tree.rebuild();
                self.right_tree.show_hidden = sh;
                self.right_tree.rebuild();
            }
            Action::RefreshPanel => {
                self.active_tree().rebuild();
            }
            Action::TogglePanels => {
                self.panels_visible = !self.panels_visible;
            }
            Action::ShowHelp => {
                self.help = Some(HelpState::new());
            }
            Action::ShowMenu => {
                self.menu = Some(MenuState::new());
            }
            Action::ShowSearchDialog => {
                let dir = self.active_panel_ref().current_dir.clone();
                self.search = Some(SearchState::new(dir));
            }
            Action::ShowInfoPanel => {
                self.show_info_panel = !self.show_info_panel;
            }
            Action::ShowAiBar => {
                self.ai_bar = Some(AiBarState::new());
            }
            Action::ShowAiPanel => {
                self.ai_panel = Some(AiPanelState::new());
            }
            Action::LaunchAiTool(tool) => {
                let (cmd, args) = tool.command();
                self.spawn_embedded_terminal(cmd, args);
            }
            // ── File operation dialogs ───────────────────────────────────
            Action::CopyDialog => {
                let sources = self.collect_selected_paths();
                if sources.is_empty() {
                    return;
                }
                let names = self.collect_selected_names();
                let dest = self.inactive_tree_root();
                let detail = format!("{} → {}", names.join(", "), dest.display());
                self.feedback
                    .ask_confirm("Copy?", detail, ConfirmAction::Copy { sources, dest });
            }
            Action::CopySameDir => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    if node.entry.name == ".." || node.entry.is_dir {
                        return;
                    }
                    let source = node.entry.path.clone();
                    let default_name = format!("{}_copy", node.entry.name);
                    self.pending_op = Some(PendingOperation::CopySameDir { source });
                    self.dialog = Some(DialogState::new_input(
                        "Copy to same directory",
                        "Enter new name:",
                        default_name,
                    ));
                }
            }
            Action::MoveDialog => {
                let sources = self.collect_selected_paths();
                if sources.is_empty() {
                    return;
                }
                let names = self.collect_selected_names();
                let dest = self.inactive_tree_root();
                let detail = format!("{} → {}", names.join(", "), dest.display());
                self.feedback
                    .ask_confirm("Move?", detail, ConfirmAction::Move { sources, dest });
            }
            Action::DeleteDialog => {
                let targets = self.collect_selected_paths();
                if targets.is_empty() {
                    return;
                }
                let names = self.collect_selected_names();
                let detail = names.join(", ");
                self.feedback
                    .ask_confirm("Delete?", detail, ConfirmAction::Delete { targets });
            }
            Action::MkDirDialog => {
                let parent = self.active_panel_ref().current_dir.clone();
                self.pending_op = Some(PendingOperation::MkDir { parent });
                self.dialog = Some(DialogState::new_input(
                    "Create directory",
                    "Enter directory name:",
                    "",
                ));
            }
            Action::RenameDialog => {
                if let Some(entry) = self.active_panel_ref().current_entry() {
                    if entry.name == ".." {
                        return;
                    }
                    let original = entry.path.clone();
                    let current_name = entry.name.clone();
                    self.pending_op = Some(PendingOperation::Rename { original });
                    self.dialog = Some(DialogState::new_input(
                        "Rename",
                        "Enter new name:",
                        current_name,
                    ));
                }
            }
            Action::CreateFileDialog => {
                let parent = self.active_panel_ref().current_dir.clone();
                self.pending_op = Some(PendingOperation::CreateFile { parent });
                self.dialog = Some(DialogState::new_input(
                    "Create file",
                    "Enter file name:",
                    "",
                ));
            }
            Action::GotoDirectoryDialog => {
                let current = self.active_tree_ref().root.to_string_lossy().to_string();
                self.pending_op = Some(PendingOperation::GotoDirectory);
                self.dialog = Some(DialogState::new_input(
                    "Go to directory",
                    "Enter path:",
                    current,
                ));
            }
            Action::CreateSymlinkDialog => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    if node.entry.name == ".." {
                        return;
                    }
                    let target = node.entry.path.clone();
                    let default_name = format!("{}_link", node.entry.name);
                    self.pending_op = Some(PendingOperation::CreateSymlink { target });
                    self.dialog = Some(DialogState::new_input(
                        "Create symlink",
                        "Enter link name:",
                        default_name,
                    ));
                }
            }
            Action::SelectByMaskDialog => {
                self.pending_op = Some(PendingOperation::SelectByMask);
                self.dialog = Some(DialogState::new_input(
                    "Select by mask",
                    "Enter pattern (e.g. *.rs, test*):",
                    "*",
                ));
            }
            Action::DeselectByMaskDialog => {
                self.pending_op = Some(PendingOperation::DeselectByMask);
                self.dialog = Some(DialogState::new_input(
                    "Deselect by mask",
                    "Enter pattern (e.g. *.rs, test*):",
                    "*",
                ));
            }
            Action::QuickSearch(ch) => {
                self.active_panel_mut().enter_quick_search(ch);
            }
            Action::QuickSearchClear => {
                self.active_panel_mut().clear_quick_search();
            }
            Action::CommandLineInput(ch) => {
                self.command_line.input_char(ch);
                self.command_line.last_typed_tick = self.tick_count;
                self.update_slash_suggestions();
            }
            Action::CommandLineBackspace => {
                self.command_line.last_typed_tick = self.tick_count;
                self.command_line.backspace();
                self.update_slash_suggestions();
            }
            // CommandLineEnterOrDir is handled in the tree block above
            Action::CommandLineExecute => {
                self.slash_suggestions = None;
                self.smart_execute_command();
            }
            Action::CommandLineHistoryUp => {
                self.command_line.history_up();
            }
            Action::CommandLineHistoryDown => {
                self.command_line.history_down();
            }
            Action::CommandLineClear => {
                self.command_line.clear();
                self.slash_suggestions = None;
            }
            Action::HistoryBack => {
                let went_back = self.active_tree().go_back();
                if went_back {
                    let new_root = self.active_tree_ref().root.clone();
                    match self.active_panel {
                        PanelSide::Left => self.left_panel.current_dir = new_root,
                        PanelSide::Right => self.right_panel.current_dir = new_root,
                    }
                } else {
                    self.feedback.info("No history to go back to".to_string());
                }
            }
            Action::HistoryForward => {
                let went_fwd = self.active_tree().go_forward();
                if went_fwd {
                    let new_root = self.active_tree_ref().root.clone();
                    match self.active_panel {
                        PanelSide::Left => self.left_panel.current_dir = new_root,
                        PanelSide::Right => self.right_panel.current_dir = new_root,
                    }
                } else {
                    self.feedback
                        .info("No history to go forward to".to_string());
                }
            }
            Action::ShowBookmarks => {
                self.bookmarks_panel = Some(BookmarkState::new(self.bookmarks.clone()));
            }
            Action::AddBookmark => {
                let dir = self.active_tree_ref().root.clone();
                let name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "/".to_string());
                // Check for duplicates
                if self.bookmarks.iter().any(|b| b.path == dir) {
                    self.feedback.info("Already bookmarked".to_string());
                } else {
                    self.bookmarks.push(Bookmark {
                        name,
                        path: dir.clone(),
                    });
                    save_bookmarks(&self.bookmarks);
                    self.feedback.info(format!("Bookmarked: {}", dir.display()));
                }
            }
            Action::ToggleFilter => {
                self.filter_active = !self.filter_active;
                if !self.filter_active {
                    self.filter_pattern.clear();
                    self.active_tree().rebuild();
                }
            }
            Action::CopyPathToClipboard => {
                let tree = self.active_tree_ref();
                // If selected, copy all selected paths; otherwise copy current
                let paths: Vec<String> = if !tree.selected.is_empty() {
                    tree.selected
                        .iter()
                        .filter_map(|&i| tree.visible_nodes.get(i))
                        .map(|n| n.entry.path.to_string_lossy().to_string())
                        .collect()
                } else if let Some(node) = tree.current_node() {
                    vec![node.entry.path.to_string_lossy().to_string()]
                } else {
                    Vec::new()
                };
                if paths.is_empty() {
                    return;
                }
                let text = paths.join("\n");
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => match clipboard.set_text(&text) {
                        Ok(()) => {
                            if paths.len() == 1 {
                                self.feedback.info(format!("Copied: {}", paths[0]));
                            } else {
                                self.feedback.info(format!("Copied {} paths", paths.len()));
                            }
                        }
                        Err(e) => {
                            self.feedback.error(format!("Clipboard: {}", e));
                        }
                    },
                    Err(e) => {
                        self.feedback.error(format!("Clipboard: {}", e));
                    }
                }
            }
            Action::OpenTerminalHere => {
                let dir = self.active_tree_ref().root.to_string_lossy().to_string();
                let result = if cfg!(target_os = "macos") {
                    std::process::Command::new("open")
                        .args(["-a", "Terminal", &dir])
                        .spawn()
                } else if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd")
                        .args(["/C", "start", "cmd", "/K", &format!("cd /d {}", dir)])
                        .spawn()
                } else {
                    // Linux: try common terminal emulators
                    std::process::Command::new("sh")
                        .args(["-c", &format!("cd '{}' && ${{TERMINAL:-xterm}} &", dir)])
                        .spawn()
                };
                match result {
                    Ok(_) => self.feedback.info("Terminal opened".to_string()),
                    Err(e) => self.feedback.error(format!("Terminal: {}", e)),
                }
            }
            Action::CopyNameToClipboard => {
                let tree = self.active_tree_ref();
                let names: Vec<String> = if !tree.selected.is_empty() {
                    tree.selected
                        .iter()
                        .filter_map(|&i| tree.visible_nodes.get(i))
                        .map(|n| n.entry.name.clone())
                        .collect()
                } else if let Some(node) = tree.current_node() {
                    vec![node.entry.name.clone()]
                } else {
                    Vec::new()
                };
                if names.is_empty() {
                    return;
                }
                let text = names.join("\n");
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => match clipboard.set_text(&text) {
                        Ok(()) => {
                            if names.len() == 1 {
                                self.feedback.info(format!("Copied name: {}", names[0]));
                            } else {
                                self.feedback.info(format!("Copied {} names", names.len()));
                            }
                        }
                        Err(e) => self.feedback.error(format!("Clipboard: {}", e)),
                    },
                    Err(e) => self.feedback.error(format!("Clipboard: {}", e)),
                }
            }
            Action::ShowQuickActions => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    let name = node.entry.name.clone();
                    let ext = node.entry.extension.as_deref();
                    let is_dir = node.entry.is_dir;
                    self.quick_actions = Some(QuickActionsState::new(name, ext, is_dir));
                }
            }
            Action::RunShellAction(cmd) => {
                self.command_line.input = cmd;
                self.smart_execute_command();
            }
            Action::ShowFuzzyFinder => {
                let root = self.active_tree_ref().root.clone();
                self.fuzzy_finder = Some(FuzzyFinderState::new(root));
            }
            Action::BatchRename => {
                let tree = self.active_tree_ref();
                let files: Vec<(PathBuf, String)> = if !tree.selected.is_empty() {
                    tree.selected
                        .iter()
                        .filter_map(|&i| tree.visible_nodes.get(i))
                        .filter(|n| !n.entry.is_dir)
                        .map(|n| (n.entry.path.clone(), n.entry.name.clone()))
                        .collect()
                } else {
                    // Use all non-dir files in current view
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
            Action::Undo => {
                if let Some(entry) = self.undo_stack.pop() {
                    match entry {
                        UndoEntry::Delete { paths } => {
                            // Can't programmatically restore from trash in a cross-platform way,
                            // but we can inform the user what was deleted
                            let names: Vec<String> = paths
                                .iter()
                                .filter_map(|p| {
                                    p.file_name().map(|n| n.to_string_lossy().to_string())
                                })
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
                                let original_dir =
                                    source.parent().unwrap_or(std::path::Path::new("/"));
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
                            Err(e) => {
                                self.feedback.error(format!("Undo rename: {}", e));
                            }
                        },
                        UndoEntry::MkDir { path } => {
                            if path.exists() && path.is_dir() {
                                let _ = std::fs::remove_dir(&path);
                                self.feedback
                                    .success("Undo: removed created directory".to_string());
                                self.active_tree().rebuild();
                            }
                        }
                        UndoEntry::CreateFile { path } => {
                            if path.exists() && path.is_file() {
                                let _ = std::fs::remove_file(&path);
                                self.feedback
                                    .success("Undo: removed created file".to_string());
                                self.active_tree().rebuild();
                            }
                        }
                    }
                } else {
                    self.feedback.info("Nothing to undo".to_string());
                }
            }
            Action::ViewArchive => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    let path = node.entry.path.clone();
                    match farx_fs::list_archive(&path) {
                        Ok(entries) => {
                            let lines: Vec<String> = entries
                                .iter()
                                .map(|e| {
                                    if e.is_dir {
                                        format!("[DIR]  {}", e.name)
                                    } else {
                                        format!("{:>8}  {}", format_size_human(e.size), e.name)
                                    }
                                })
                                .collect();
                            let title =
                                format!("Archive: {} ({} entries)", node.entry.name, entries.len());
                            self.feedback.show_output(&title, lines.join("\n"));
                        }
                        Err(e) => {
                            self.feedback.error(format!("Archive: {}", e));
                        }
                    }
                }
            }
            Action::ExtractArchive => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    let path = node.entry.path.clone();
                    if !farx_fs::is_archive(&path) {
                        self.feedback.error("Not a supported archive".to_string());
                    } else {
                        let dest = self.inactive_tree_root();
                        match farx_fs::extract_archive(&path, &dest) {
                            Ok(count) => {
                                self.feedback.success(format!(
                                    "Extracted {} entries to {}",
                                    count,
                                    dest.display()
                                ));
                                self.left_tree.rebuild();
                                self.right_tree.rebuild();
                            }
                            Err(e) => {
                                self.feedback.error(format!("Extract: {}", e));
                            }
                        }
                    }
                }
            }
            Action::CompressSelection => {
                let paths = self.collect_selected_paths();
                if paths.is_empty() {
                    self.feedback
                        .error("No files selected to compress".to_string());
                } else {
                    let names = self.collect_selected_names();
                    let archive_name = if names.len() == 1 {
                        format!("{}.zip", names[0])
                    } else {
                        "archive.zip".to_string()
                    };
                    let dest = self.active_tree_ref().root.join(&archive_name);
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    match farx_fs::compress_to_zip(&refs, &dest) {
                        Ok(count) => {
                            self.feedback
                                .success(format!("Compressed {} files to {}", count, archive_name));
                            self.active_tree().rebuild();
                        }
                        Err(e) => {
                            self.feedback.error(format!("Compress: {}", e));
                        }
                    }
                }
            }
            Action::SshBrowse(target) => {
                // Parse target: user@host:/path or user@host (defaults to ~)
                let (host_part, remote_path) = if let Some(idx) = target.find(':') {
                    (&target[..idx], &target[idx + 1..])
                } else {
                    (target.as_str(), "~")
                };

                self.feedback
                    .info(format!("Connecting to {}...", host_part));

                // Use system ssh to list remote directory
                let cmd = format!(
                    "ssh -o ConnectTimeout=5 -o BatchMode=yes {} ls -lahF {}",
                    host_part, remote_path
                );
                let output = std::process::Command::new("sh").args(["-c", &cmd]).output();

                match output {
                    Ok(out) if out.status.success() => {
                        let listing = String::from_utf8_lossy(&out.stdout).to_string();
                        let title = format!("SSH: {}:{}", host_part, remote_path);
                        self.feedback.show_output(&title, listing);
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        self.feedback
                            .error(format!("SSH failed: {}", stderr.trim()));
                    }
                    Err(e) => {
                        self.feedback.error(format!("SSH: {}", e));
                    }
                }
            }
            Action::ShowFileStats => {
                if let Some(node) = self.active_tree_ref().current_node() {
                    if node.entry.is_dir || node.entry.name == ".." {
                        self.feedback.info("Select a file for stats".to_string());
                    } else {
                        match std::fs::read(&node.entry.path) {
                            Ok(bytes) => {
                                let size = bytes.len();
                                let is_text = !bytes[..size.min(512)].contains(&0);
                                let mut lines = Vec::new();
                                lines.push(format!("File: {}", node.entry.name));
                                lines.push(format!("Size: {} bytes", size));
                                if is_text {
                                    let text = String::from_utf8_lossy(&bytes);
                                    let line_count = text.lines().count();
                                    let word_count: usize =
                                        text.lines().map(|l| l.split_whitespace().count()).sum();
                                    let char_count = text.chars().count();
                                    lines.push(format!("Lines: {}", line_count));
                                    lines.push(format!("Words: {}", word_count));
                                    lines.push(format!("Characters: {}", char_count));
                                } else {
                                    lines.push("Binary file".to_string());
                                }
                                self.feedback
                                    .show_output("File Statistics", lines.join("\n"));
                            }
                            Err(e) => self.feedback.error(format!("Stats: {}", e)),
                        }
                    }
                }
            }
            Action::ShowChecksums => {
                let tree = self.active_tree_ref();
                let files: Vec<(String, std::path::PathBuf)> = if !tree.selected.is_empty() {
                    tree.selected
                        .iter()
                        .filter_map(|&i| tree.visible_nodes.get(i))
                        .filter(|n| !n.entry.is_dir)
                        .map(|n| (n.entry.name.clone(), n.entry.path.clone()))
                        .collect()
                } else if let Some(node) = tree.current_node() {
                    if !node.entry.is_dir {
                        vec![(node.entry.name.clone(), node.entry.path.clone())]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                if files.is_empty() {
                    self.feedback.error("No files selected".to_string());
                } else {
                    use sha2::{Digest, Sha256};
                    let mut lines = Vec::new();
                    for (name, path) in &files {
                        match std::fs::read(path) {
                            Ok(data) => {
                                let mut hasher = Sha256::new();
                                hasher.update(&data);
                                let hash = format!("{:x}", hasher.finalize());
                                lines.push(format!("SHA-256: {}", hash));
                                lines.push(format!("  File: {}", name));
                                lines.push(format!(
                                    "  Size: {}",
                                    format_size_human(data.len() as u64)
                                ));
                                lines.push(String::new());
                            }
                            Err(e) => {
                                lines.push(format!("{}: error — {}", name, e));
                                lines.push(String::new());
                            }
                        }
                    }
                    self.feedback.show_output("Checksums", lines.join("\n"));
                }
            }
            Action::ChmodDialog => {
                #[cfg(unix)]
                {
                    if let Some(node) = self.active_tree_ref().current_node() {
                        if let Some(mode) = node.entry.mode {
                            self.chmod_dialog =
                                Some(ChmodDialogState::new(node.entry.path.clone(), mode));
                        } else {
                            self.feedback
                                .error("Cannot read file permissions".to_string());
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    self.feedback
                        .error("Chmod is only available on Unix systems".to_string());
                }
            }
            Action::DiffFiles => {
                // Get current file from left and right panels
                let left_file = self.left_tree.current_node().and_then(|n| {
                    if !n.entry.is_dir {
                        Some(n.entry.path.clone())
                    } else {
                        None
                    }
                });
                let right_file = self.right_tree.current_node().and_then(|n| {
                    if !n.entry.is_dir {
                        Some(n.entry.path.clone())
                    } else {
                        None
                    }
                });
                match (left_file, right_file) {
                    (Some(left), Some(right)) => match DiffViewState::new(left, right) {
                        Ok(dv) => self.diff_view = Some(dv),
                        Err(e) => self.feedback.error(format!("Diff failed: {}", e)),
                    },
                    _ => {
                        self.feedback
                            .error("Place cursor on a file in each panel to diff".to_string());
                    }
                }
            }
            Action::NewTab => {
                let root = self.active_tree_ref().root.clone();
                let show_hidden = self.config.general.show_hidden_files;
                match self.active_panel {
                    PanelSide::Left => self.left_tree.new_tab(root, show_hidden),
                    PanelSide::Right => self.right_tree.new_tab(root, show_hidden),
                }
            }
            Action::CloseTab => {
                let closed = match self.active_panel {
                    PanelSide::Left => self.left_tree.close_tab(),
                    PanelSide::Right => self.right_tree.close_tab(),
                };
                if !closed {
                    self.feedback.info("Cannot close the last tab".to_string());
                }
            }
            Action::NextTab => match self.active_panel {
                PanelSide::Left => self.left_tree.next_tab(),
                PanelSide::Right => self.right_tree.next_tab(),
            },
            Action::PrevTab => match self.active_panel {
                PanelSide::Left => self.left_tree.prev_tab(),
                PanelSide::Right => self.right_tree.prev_tab(),
            },
            Action::SwitchTab(idx) => match self.active_panel {
                PanelSide::Left => self.left_tree.switch_to(idx),
                PanelSide::Right => self.right_tree.switch_to(idx),
            },
            Action::FindDuplicates => {
                let root = self.active_tree_ref().root.clone();
                self.feedback.info("Scanning for duplicates...".to_string());
                match farx_fs::find_duplicates(&root, 8) {
                    Ok(groups) => {
                        if groups.is_empty() {
                            self.feedback.info("No duplicate files found".to_string());
                        } else {
                            let total_waste: u64 = groups
                                .iter()
                                .map(|g| g.size * (g.paths.len() as u64 - 1))
                                .sum();
                            let mut lines = Vec::new();
                            lines.push(format!(
                                "{} duplicate groups, {} reclaimable",
                                groups.len(),
                                format_size_human(total_waste)
                            ));
                            lines.push(String::new());
                            for (i, group) in groups.iter().take(50).enumerate() {
                                lines.push(format!(
                                    "Group {} — {} each, {} copies:",
                                    i + 1,
                                    format_size_human(group.size),
                                    group.paths.len()
                                ));
                                for path in &group.paths {
                                    lines.push(format!("  {}", path.display()));
                                }
                                lines.push(String::new());
                            }
                            self.feedback
                                .show_output("Duplicate Files", lines.join("\n"));
                        }
                    }
                    Err(e) => {
                        self.feedback.error(format!("Duplicates: {}", e));
                    }
                }
            }
            Action::ShowTreemap => {
                self.show_treemap();
            }
            Action::CalculateDirSize => {
                self.calculate_dir_size();
            }
            Action::TouchFile => {
                let paths = self.collect_selected_paths();
                if paths.is_empty() {
                    return;
                }
                let now = std::time::SystemTime::now();
                let mut ok = 0;
                let mut fail = 0;
                for path in &paths {
                    if path.exists() {
                        // Update mtime by opening and setting modified time
                        match std::fs::File::options().write(true).open(path) {
                            Ok(f) => match f.set_modified(now) {
                                Ok(()) => ok += 1,
                                Err(_) => fail += 1,
                            },
                            Err(_) => fail += 1,
                        }
                    } else {
                        // Create the file (like Unix touch)
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
            Action::CompareDirectories => {
                self.compare_directories();
            }
            Action::ShowRecentDirectories => {
                let tree = self.active_tree_ref();
                let mut dirs: Vec<String> = Vec::new();
                // Current directory
                dirs.push(format!("  * {}", tree.root.display()));
                // Back stack (most recent first)
                for dir in tree.history_back.iter().rev() {
                    dirs.push(format!("    {}", dir.display()));
                }
                if dirs.len() <= 1 {
                    self.feedback.info("No directory history yet".to_string());
                } else {
                    self.feedback
                        .show_output("Recent Directories", dirs.join("\n"));
                }
            }
            Action::SortByName => {
                self.toggle_sort(SortField::Name);
            }
            Action::SortByExtension => {
                self.toggle_sort(SortField::Extension);
            }
            Action::SortBySize => {
                self.toggle_sort(SortField::Size);
            }
            Action::SortByDate => {
                self.toggle_sort(SortField::Modified);
            }
            Action::ShowPluginCommands => {
                // Reuse the /plugin slash command logic
                if let Some(ref engine) = self.plugin_engine {
                    let cmds = engine.list_commands();
                    if cmds.is_empty() {
                        self.feedback.info(
                            "No plugins loaded. Place .lua files in ~/.config/farx/plugins/"
                                .to_string(),
                        );
                    } else {
                        let lines: Vec<String> = cmds
                            .iter()
                            .map(|c| {
                                format!("  /{} — {} ({})", c.name, c.description, c.plugin_file)
                            })
                            .collect();
                        self.feedback.show_output("Plugins", lines.join("\n"));
                    }
                } else {
                    self.feedback
                        .error("Plugin engine not available".to_string());
                }
            }
            Action::ShowDriveMenu(_) => {
                // Drive/volume menu — on Unix, show mount points
                let output = std::process::Command::new("df")
                    .args(["-h", "--output=target,size,avail,pcent"])
                    .output()
                    .or_else(|_| {
                        // macOS df doesn't support --output
                        std::process::Command::new("df").args(["-h"]).output()
                    });
                match output {
                    Ok(out) => {
                        let text = String::from_utf8_lossy(&out.stdout).to_string();
                        self.feedback.show_output("Volumes", text);
                    }
                    Err(e) => self.feedback.error(format!("df: {}", e)),
                }
            }
            _ => {
                // Other actions not yet implemented
            }
        }
    }
}
