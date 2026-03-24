use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use farx_core::{Action, AppConfig, KeyMap, PanelSide, PanelState, TreeState};

use farx_core::SortField;

use crate::components::ai_bar::{render_ai_bar, AiBarAction, AiBarState};
use crate::components::dialog::{render_dialog, DialogResult, DialogState};
use crate::components::editor::{render_editor, EditorAction, EditorState};
use crate::components::help::{render_help, HelpState};
use crate::components::info_panel::{render_info_panel, InfoPanelData};
use crate::components::menu::{render_menu, MenuAction, MenuState};
use crate::components::search::{render_search, SearchAction, SearchState};
use crate::components::tree_panel::render_tree_panel;
use crate::components::viewer::{render_viewer, ViewerAction, ViewerState};
use crate::components::{command_line, fn_bar, panel};
use crate::components::command_line::CommandLineState;
use crate::theme::Theme;

/// Tracks which dialog-triggering action opened the current dialog,
/// so we know what file operation to perform when the user confirms.
#[derive(Debug, Clone)]
enum PendingOperation {
    Copy { sources: Vec<PathBuf>, dest_dir: PathBuf },
    Move { sources: Vec<PathBuf>, dest_dir: PathBuf },
    Delete { targets: Vec<PathBuf> },
    MkDir { parent: PathBuf },
    Rename { original: PathBuf },
    CreateFile { parent: PathBuf },
}

/// Main application state that owns panels, config, and the render loop.
pub struct App {
    /// Whether the application is still running.
    pub running: bool,
    /// Which panel is currently active / focused.
    pub active_panel: PanelSide,
    /// Left file panel state.
    pub left_panel: PanelState,
    /// Right file panel state.
    pub right_panel: PanelState,
    /// Command line input state.
    pub command_line: CommandLineState,
    /// Whether the dual panels are visible (Ctrl+O toggles).
    pub panels_visible: bool,
    /// Application configuration.
    pub config: AppConfig,
    /// Key bindings.
    pub keymap: KeyMap,
    /// Visual theme.
    pub theme: Theme,
    /// Currently open modal dialog, if any.
    pub dialog: Option<DialogState>,
    /// The pending file operation associated with the current dialog.
    pending_op: Option<PendingOperation>,
    /// File viewer state (F3).
    pub viewer: Option<ViewerState>,
    /// Help screen state (F1).
    pub help: Option<HelpState>,
    /// AI bar state (Ctrl+Space).
    pub ai_bar: Option<AiBarState>,
    /// AI agent for processing queries.
    ai_agent: farx_ai::AiAgent,
    /// Tokio runtime handle for async AI queries.
    ai_pending_response: Option<tokio::sync::oneshot::Receiver<String>>,
    /// Editor state (F4).
    pub editor: Option<EditorState>,
    /// Menu bar state (F9).
    pub menu: Option<MenuState>,
    /// Search dialog state (Alt+F7).
    pub search: Option<SearchState>,
    /// Whether to show info panel instead of inactive panel (Ctrl+L).
    pub show_info_panel: bool,
    /// Command output to display.
    pub command_output: Option<String>,
    /// Tree view state for the left panel.
    pub left_tree: TreeState,
    /// Tree view state for the right panel.
    pub right_tree: TreeState,
}

impl App {
    /// Create a new App, loading directory contents for both panels.
    ///
    /// The left panel starts in the current working directory and the right
    /// panel starts in the user's home directory.
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let cwd2 = cwd.clone();
        let home = dirs::home_dir().unwrap_or_else(|| cwd.clone());
        let show_hidden = config.general.show_hidden_files;

        let home2 = home.clone();
        let mut left = PanelState::new(PanelSide::Left, cwd);
        let mut right = PanelState::new(PanelSide::Right, home);

        // Load initial directory contents
        Self::refresh_panel(&mut left, show_hidden);
        Self::refresh_panel(&mut right, show_hidden);

        let ai_agent = farx_ai::AiAgent::new(
            &config.ai.provider,
            config.ai.base_url.clone(),
            config.ai.model.clone(),
            config.ai.max_tokens,
            &config.ai.api_key_env,
        );

        Ok(Self {
            running: true,
            active_panel: PanelSide::Left,
            left_panel: left,
            right_panel: right,
            command_line: CommandLineState::new(),
            panels_visible: true,
            keymap: KeyMap::far_defaults(),
            theme: Theme::by_name(&config.ui.theme),
            config,
            dialog: None,
            pending_op: None,
            viewer: None,
            help: None,
            ai_bar: None,
            ai_agent,
            ai_pending_response: None,
            editor: None,
            menu: None,
            search: None,
            show_info_panel: false,
            command_output: None,
            left_tree: {
                let mut t = TreeState::new(cwd2);
                t.show_hidden = show_hidden;
                t
            },
            right_tree: {
                let mut t = TreeState::new(home2);
                t.show_hidden = show_hidden;
                t
            },
        })
    }

    /// Re-read the directory listing for a panel and sort the entries.
    fn refresh_panel(panel: &mut PanelState, show_hidden: bool) {
        if let Ok(entries) = farx_fs::read_directory(&panel.current_dir, show_hidden) {
            panel.entries = entries;
            panel.sort_entries();
        }
    }

    /// Refresh both panels.
    fn refresh_both_panels(&mut self) {
        let show_hidden = self.config.general.show_hidden_files;
        Self::refresh_panel(&mut self.left_panel, show_hidden);
        Self::refresh_panel(&mut self.right_panel, show_hidden);
    }

    /// Get a mutable reference to the currently active panel.
    pub fn active_panel_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    /// Get the active tree.
    fn active_tree(&mut self) -> &mut TreeState {
        match self.active_panel {
            PanelSide::Left => &mut self.left_tree,
            PanelSide::Right => &mut self.right_tree,
        }
    }

    /// Get a reference to the currently active panel.
    pub fn active_panel_ref(&self) -> &PanelState {
        match self.active_panel {
            PanelSide::Left => &self.left_panel,
            PanelSide::Right => &self.right_panel,
        }
    }

    /// Get a reference to the currently inactive panel.
    pub fn inactive_panel(&self) -> &PanelState {
        match self.active_panel {
            PanelSide::Left => &self.right_panel,
            PanelSide::Right => &self.left_panel,
        }
    }

    /// Collect the paths of selected files (or the current file if nothing is selected).
    /// Skips the ".." entry.
    fn collect_selected_paths(&self) -> Vec<PathBuf> {
        let panel = self.active_panel_ref();
        let selected = panel.selected_entries();
        if selected.is_empty() {
            // Use current entry if nothing is selected
            if let Some(entry) = panel.current_entry() {
                if entry.name != ".." {
                    return vec![entry.path.clone()];
                }
            }
            Vec::new()
        } else {
            selected
                .into_iter()
                .filter(|e| e.name != "..")
                .map(|e| e.path.clone())
                .collect()
        }
    }

    /// Collect display names for the selected/current files.
    fn collect_selected_names(&self) -> Vec<String> {
        let panel = self.active_panel_ref();
        let selected = panel.selected_entries();
        if selected.is_empty() {
            if let Some(entry) = panel.current_entry() {
                if entry.name != ".." {
                    return vec![entry.name.clone()];
                }
            }
            Vec::new()
        } else {
            selected
                .into_iter()
                .filter(|e| e.name != "..")
                .map(|e| e.name.clone())
                .collect()
        }
    }

    /// Map a key event to an action via the keymap, or send it to the active modal.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        // Priority: editor > viewer > help > menu > search > ai_bar > dialog > panel

        // Editor is full-screen
        if let Some(ref mut editor) = self.editor {
            match editor.handle_key_event(key) {
                EditorAction::Close | EditorAction::SaveAndClose => {
                    self.editor = None;
                    self.refresh_both_panels();
                }
                EditorAction::None => {}
            }
            return Action::Noop;
        }

        // Viewer is full-screen
        if let Some(ref mut viewer) = self.viewer {
            match viewer.handle_key_event(key) {
                ViewerAction::Close => { self.viewer = None; }
                ViewerAction::None => {}
            }
            return Action::Noop;
        }

        // Help screen
        if let Some(ref mut help) = self.help {
            help.handle_key_event(key);
            if !help.active {
                self.help = None;
            }
            return Action::Noop;
        }

        // Menu bar
        if let Some(ref mut menu) = self.menu {
            let action = menu.handle_key_event(key);
            if !menu.active {
                self.menu = None;
            }
            self.handle_menu_action(action);
            return Action::Noop;
        }

        // Search dialog
        if let Some(ref mut search) = self.search {
            let action = search.handle_key_event(key);
            if !search.active {
                self.search = None;
            }
            if let SearchAction::GoTo(path) = action {
                let show_hidden = self.config.general.show_hidden_files;
                let panel = self.active_panel_mut();
                panel.current_dir = path;
                panel.cursor = 0;
                panel.scroll_offset = 0;
                panel.selected.clear();
                Self::refresh_panel(panel, show_hidden);
            }
            return Action::Noop;
        }

        // AI bar
        if let Some(ref mut ai_bar) = self.ai_bar {
            match ai_bar.handle_key_event(key) {
                AiBarAction::Close => { self.ai_bar = None; }
                AiBarAction::Submit(query) => {
                    self.submit_ai_query(query);
                }
                AiBarAction::None => {}
            }
            return Action::Noop;
        }

        // Dialog
        if let Some(ref mut dialog) = self.dialog {
            dialog.handle_key_event(key);
            if dialog.is_resolved() {
                let result = dialog.result.clone();
                let pending = self.pending_op.take();
                self.dialog = None;
                self.handle_dialog_result(result, pending);
            }
            return Action::Noop;
        }

        // If command line has input, intercept some keys for command line editing
        if !self.command_line.input.is_empty() {
            use crossterm::event::{KeyCode, KeyModifiers};
            match (key.code, key.modifiers) {
                (KeyCode::Up, KeyModifiers::NONE) => return Action::CommandLineHistoryUp,
                (KeyCode::Down, KeyModifiers::NONE) => return Action::CommandLineHistoryDown,
                (KeyCode::Esc, _) => return Action::CommandLineClear,
                (KeyCode::Left, KeyModifiers::NONE) => {
                    self.command_line.cursor_pos = self.command_line.cursor_pos.saturating_sub(1);
                    return Action::Noop;
                }
                (KeyCode::Right, KeyModifiers::NONE) => {
                    self.command_line.cursor_pos =
                        (self.command_line.cursor_pos + 1).min(self.command_line.input.len());
                    return Action::Noop;
                }
                _ => {}
            }
        }

        self.keymap.resolve_panel(&key)
    }

    fn handle_menu_action(&mut self, action: MenuAction) {
        let show_hidden = self.config.general.show_hidden_files;
        match action {
            MenuAction::SortByName => {
                self.active_panel_mut().sort_field = SortField::Name;
                self.active_panel_mut().sort_entries();
            }
            MenuAction::SortByExtension => {
                self.active_panel_mut().sort_field = SortField::Extension;
                self.active_panel_mut().sort_entries();
            }
            MenuAction::SortBySize => {
                self.active_panel_mut().sort_field = SortField::Size;
                self.active_panel_mut().sort_entries();
            }
            MenuAction::SortByDate => {
                self.active_panel_mut().sort_field = SortField::Modified;
                self.active_panel_mut().sort_entries();
            }
            MenuAction::ToggleHidden => {
                self.config.general.show_hidden_files = !self.config.general.show_hidden_files;
                self.refresh_both_panels();
            }
            MenuAction::Refresh => {
                Self::refresh_panel(self.active_panel_mut(), show_hidden);
            }
            MenuAction::ViewFile => self.dispatch(Action::ViewFile),
            MenuAction::EditFile => self.dispatch(Action::EditFile),
            MenuAction::CopyFile => self.dispatch(Action::CopyDialog),
            MenuAction::MoveFile => self.dispatch(Action::MoveDialog),
            MenuAction::DeleteFile => self.dispatch(Action::DeleteDialog),
            MenuAction::MkDir => self.dispatch(Action::MkDirDialog),
            MenuAction::FindFiles => self.dispatch(Action::ShowSearchDialog),
            MenuAction::ShowAiBar => self.dispatch(Action::ShowAiBar),
            MenuAction::SwapPanels => {
                std::mem::swap(&mut self.left_panel, &mut self.right_panel);
                self.left_panel.side = PanelSide::Left;
                self.right_panel.side = PanelSide::Right;
            }
            MenuAction::ToggleFnBar => {
                self.config.ui.show_fn_bar = !self.config.ui.show_fn_bar;
            }
            MenuAction::None | MenuAction::Close => {}
        }
    }

    /// Submit an AI query in the background.
    fn submit_ai_query(&mut self, query: String) {
        let current_dir = self.active_panel_ref().current_dir.clone();
        let entries: Vec<(String, bool, u64)> = self.active_panel_ref()
            .entries.iter()
            .map(|e| (e.name.clone(), e.is_dir, e.size))
            .collect();
        let files_context = farx_ai::AiAgent::build_files_context(&entries);

        let agent = farx_ai::AiAgent::new(
            &self.config.ai.provider,
            self.ai_agent.base_url().to_string(),
            self.ai_agent.model().to_string(),
            self.ai_agent.max_tokens(),
            &self.config.ai.api_key_env,
        );

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.ai_pending_response = Some(rx);

        tokio::spawn(async move {
            let result = agent.query(&query, &current_dir, &files_context).await;
            let response = match result {
                Ok(text) => text,
                Err(e) => format!("Error: {}", e),
            };
            let _ = tx.send(response);
        });
    }

    /// Check for completed AI responses (called from tick).
    pub fn check_ai_response(&mut self) {
        if let Some(ref mut rx) = self.ai_pending_response {
            match rx.try_recv() {
                Ok(response) => {
                    if let Some(ref mut ai_bar) = self.ai_bar {
                        ai_bar.set_response(response);
                    }
                    self.ai_pending_response = None;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    // Still waiting
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    if let Some(ref mut ai_bar) = self.ai_bar {
                        ai_bar.set_response("AI query was cancelled.".to_string());
                    }
                    self.ai_pending_response = None;
                }
            }
        }
    }

    /// Smart command execution: detects whether the input is a shell command or
    /// natural language, and routes accordingly.
    ///
    /// Heuristic: if the input starts with a known command/path prefix, or contains
    /// shell operators, treat it as a shell command. Otherwise treat as AI query.
    fn smart_execute_command(&mut self) {
        let input = self.command_line.take_input();
        if input.is_empty() {
            return;
        }

        // Save to history regardless
        self.command_line.history.push(input.clone());

        if Self::looks_like_shell_command(&input) {
            // Execute as shell command
            let output = if cfg!(windows) {
                std::process::Command::new("cmd")
                    .args(["/C", &input])
                    .current_dir(&self.active_panel_ref().current_dir)
                    .output()
            } else {
                std::process::Command::new("sh")
                    .args(["-c", &input])
                    .current_dir(&self.active_panel_ref().current_dir)
                    .output()
            };

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let result = if stderr.is_empty() {
                        stdout
                    } else if stdout.is_empty() {
                        stderr
                    } else {
                        format!("{}\n{}", stdout, stderr)
                    };
                    let result = result.trim().to_string();
                    if !result.is_empty() {
                        self.dialog = Some(DialogState::new_message("Command Output", result));
                    }
                }
                Err(e) => {
                    self.show_error("Command Error", &format!("{}", e));
                }
            }
            self.refresh_both_panels();
        } else {
            // Natural language — route to AI bar
            self.ai_bar = Some(AiBarState::new());
            if let Some(ref mut ai_bar) = self.ai_bar {
                ai_bar.input = input.clone();
                ai_bar.cursor_pos = input.len();
                ai_bar.thinking = true;
            }
            self.submit_ai_query(input);
        }
    }

    /// Heuristic to detect shell commands vs natural language.
    fn looks_like_shell_command(input: &str) -> bool {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Starts with common shell prefixes
        let first_word = trimmed.split_whitespace().next().unwrap_or("");

        // Absolute or relative path
        if first_word.starts_with('/') || first_word.starts_with("./") || first_word.starts_with("~/") {
            return true;
        }

        // Contains shell operators
        if trimmed.contains('|') || trimmed.contains('>') || trimmed.contains('<')
            || trimmed.contains("&&") || trimmed.contains("||") || trimmed.contains(';')
        {
            return true;
        }

        // Starts with common command names
        const SHELL_COMMANDS: &[&str] = &[
            "ls", "cd", "cp", "mv", "rm", "mkdir", "rmdir", "cat", "head", "tail",
            "grep", "find", "sed", "awk", "sort", "uniq", "wc", "echo", "printf",
            "touch", "chmod", "chown", "chgrp", "ln", "pwd", "env", "export",
            "which", "whereis", "whoami", "date", "cal", "df", "du", "free",
            "top", "ps", "kill", "tar", "zip", "unzip", "gzip", "gunzip",
            "curl", "wget", "ssh", "scp", "rsync", "git", "docker", "make",
            "npm", "yarn", "pnpm", "cargo", "rustc", "python", "python3", "pip",
            "node", "ruby", "go", "java", "javac", "gcc", "g++", "clang",
            "brew", "apt", "yum", "dnf", "pacman", "snap", "flatpak",
            "systemctl", "journalctl", "sudo", "su", "man", "less", "more",
            "vi", "vim", "nano", "emacs", "code", "open", "xdg-open",
            "clear", "reset", "history", "alias", "unalias", "set", "unset",
            "test", "true", "false", "yes", "no", "tee", "xargs", "diff",
            "patch", "file", "stat", "md5", "sha256sum", "base64",
        ];

        if SHELL_COMMANDS.contains(&first_word) {
            return true;
        }

        // Environment variable assignment (FOO=bar)
        if first_word.contains('=') && !first_word.starts_with('=') {
            return true;
        }

        // If first word contains a dot and looks like a script (./foo.sh, script.py)
        if first_word.contains('.') && (first_word.ends_with(".sh") || first_word.ends_with(".py")
            || first_word.ends_with(".rb") || first_word.ends_with(".js")
            || first_word.ends_with(".pl"))
        {
            return true;
        }

        // Default: treat as natural language (AI query)
        false
    }

    /// Process the result of a closed dialog and execute the corresponding file operation.
    fn handle_dialog_result(&mut self, result: DialogResult, pending: Option<PendingOperation>) {
        match result {
            DialogResult::Confirm(input_value) => {
                if let Some(op) = pending {
                    self.execute_pending_operation(op, input_value);
                }
            }
            DialogResult::Cancel | DialogResult::Pending => {
                // Do nothing, dialog was cancelled or somehow still pending
            }
        }
    }

    /// Execute the file operation associated with a confirmed dialog.
    fn execute_pending_operation(&mut self, op: PendingOperation, input_value: Option<String>) {
        let result = match op {
            PendingOperation::Copy { sources, dest_dir } => {
                let mut last_err = None;
                for source in &sources {
                    if let Err(e) = farx_fs::copy_entry(source, &dest_dir) {
                        last_err = Some(e);
                    }
                }
                last_err.map(Err).unwrap_or(Ok(()))
            }
            PendingOperation::Move { sources, dest_dir } => {
                let mut last_err = None;
                for source in &sources {
                    if let Err(e) = farx_fs::move_entry(source, &dest_dir) {
                        last_err = Some(e);
                    }
                }
                last_err.map(Err).unwrap_or(Ok(()))
            }
            PendingOperation::Delete { targets } => {
                let mut last_err = None;
                for target in &targets {
                    if let Err(e) = farx_fs::delete_entry(target, false) {
                        last_err = Some(e);
                    }
                }
                last_err.map(Err).unwrap_or(Ok(()))
            }
            PendingOperation::MkDir { parent } => {
                if let Some(name) = input_value {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let dir_path = parent.join(name);
                    farx_fs::create_directory(&dir_path)
                } else {
                    return;
                }
            }
            PendingOperation::Rename { original } => {
                if let Some(new_name) = input_value {
                    let new_name = new_name.trim();
                    if new_name.is_empty() {
                        return;
                    }
                    if let Some(parent) = original.parent() {
                        let new_path = parent.join(new_name);
                        farx_fs::rename_entry(&original, &new_path)
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
            PendingOperation::CreateFile { parent } => {
                if let Some(name) = input_value {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let file_path = parent.join(name);
                    // Create parent dirs if needed, then create the file
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
                } else {
                    return;
                }
            }
        };

        // Refresh panels after any file operation
        self.refresh_both_panels();

        // Show error dialog if the operation failed
        if let Err(e) = result {
            self.show_error("Error", &format!("{e}"));
        }
    }

    /// Show an error dialog.
    fn show_error(&mut self, title: &str, message: &str) {
        self.dialog = Some(DialogState::new_error(title, message));
        self.pending_op = None;
    }

    /// Execute an action, updating application state accordingly.
    pub fn dispatch(&mut self, action: Action) {
        // Both panels use tree view — route navigation through the active tree
        match &action {
            Action::CursorUp => { self.active_tree().move_cursor(-1); return; }
            Action::CursorDown => { self.active_tree().move_cursor(1); return; }
            Action::CursorPageUp => { self.active_tree().move_cursor(-20); return; }
            Action::CursorPageDown => { self.active_tree().move_cursor(20); return; }
            Action::CursorHome => { self.active_tree().move_cursor_to(0); return; }
            Action::CursorEnd => {
                let last = self.active_tree().visible_nodes.len().saturating_sub(1);
                self.active_tree().move_cursor_to(last);
                return;
            }
            Action::TreeExpand => {
                self.active_tree().expand();
                return;
            }
            Action::TreeCollapse => {
                self.active_tree().collapse();
                return;
            }
            Action::EnterDirectory | Action::CommandLineEnterOrDir => {
                if matches!(action, Action::CommandLineEnterOrDir) && !self.command_line.input.is_empty() {
                    // Command line has input — execute it
                    self.smart_execute_command();
                    return;
                }
                // Enter on tree node: toggle expand/collapse dirs, open files
                let tree = self.active_tree();
                if let Some(node) = tree.current_node() {
                    if node.entry.is_dir {
                        if node.expanded {
                            tree.collapse();
                        } else {
                            tree.expand();
                        }
                    } else {
                        let path = node.entry.path.clone();
                        match ViewerState::open(&path) {
                            Ok(vs) => { self.viewer = Some(vs); }
                            Err(e) => { self.show_error("View", &format!("{}", e)); }
                        }
                    }
                }
                return;
            }
            Action::ParentDirectory => {
                self.active_tree().collapse();
                return;
            }
            Action::ToggleSelect => {
                self.active_tree().toggle_select();
                return;
            }
            Action::SelectUp => {
                self.active_tree().toggle_select();
                self.active_tree().move_cursor(-1);
                return;
            }
            Action::SelectDown => {
                self.active_tree().toggle_select();
                self.active_tree().move_cursor(1);
                return;
            }
            Action::ViewFile => {
                if let Some(node) = self.active_tree().current_node() {
                    if !node.entry.is_dir {
                        let path = node.entry.path.clone();
                        match ViewerState::open(&path) {
                            Ok(vs) => { self.viewer = Some(vs); }
                            Err(e) => { self.show_error("View", &format!("{}", e)); }
                        }
                    }
                }
                return;
            }
            Action::EditFile => {
                if let Some(node) = self.active_tree().current_node() {
                    if !node.entry.is_dir {
                        let path = node.entry.path.clone();
                        match EditorState::open(&path) {
                            Ok(es) => { self.editor = Some(es); }
                            Err(e) => { self.show_error("Edit", &format!("{}", e)); }
                        }
                    }
                }
                return;
            }
            _ => {} // fall through to other actions
        }

        match action {
            Action::Quit => {
                self.running = false;
            }
            Action::SwitchPanel => {
                self.active_panel = match self.active_panel {
                    PanelSide::Left => PanelSide::Right,
                    PanelSide::Right => PanelSide::Left,
                };
            }
            Action::GotoRoot => {
                let root = if cfg!(windows) {
                    PathBuf::from("C:\\")
                } else {
                    PathBuf::from("/")
                };
                self.active_tree().set_root(root);
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
            // ── File operation dialogs ───────────────────────────────────
            Action::CopyDialog => {
                let sources = self.collect_selected_paths();
                let names = self.collect_selected_names();
                if sources.is_empty() {
                    return;
                }
                let dest_dir = self.inactive_panel().current_dir.clone();
                let count = sources.len();
                let message = format!(
                    "Copy {} file(s) to {}?",
                    count,
                    dest_dir.display()
                );
                self.pending_op = Some(PendingOperation::Copy {
                    sources,
                    dest_dir,
                });
                self.dialog = Some(DialogState::new_confirm("Copy", message, names));
            }
            Action::MoveDialog => {
                let sources = self.collect_selected_paths();
                let names = self.collect_selected_names();
                if sources.is_empty() {
                    return;
                }
                let dest_dir = self.inactive_panel().current_dir.clone();
                let count = sources.len();
                let message = format!(
                    "Move {} file(s) to {}?",
                    count,
                    dest_dir.display()
                );
                self.pending_op = Some(PendingOperation::Move {
                    sources,
                    dest_dir,
                });
                self.dialog = Some(DialogState::new_confirm("Move", message, names));
            }
            Action::DeleteDialog => {
                let targets = self.collect_selected_paths();
                let names = self.collect_selected_names();
                if targets.is_empty() {
                    return;
                }
                let count = targets.len();
                let message = format!("Delete {} file(s)?", count);
                self.pending_op = Some(PendingOperation::Delete { targets });
                self.dialog = Some(DialogState::new_confirm("Delete", message, names));
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
            Action::QuickSearch(ch) => {
                self.active_panel_mut().enter_quick_search(ch);
            }
            Action::QuickSearchClear => {
                self.active_panel_mut().clear_quick_search();
            }
            Action::CommandLineInput(ch) => {
                self.command_line.input_char(ch);
            }
            Action::CommandLineBackspace => {
                self.command_line.backspace();
            }
            // CommandLineEnterOrDir is handled in the tree block above
            Action::CommandLineExecute => {
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
            }
            _ => {
                // Other actions not yet implemented
            }
        }
    }

    /// Render the entire application UI into the given frame.
    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Full-screen modals first
        if let Some(ref editor) = self.editor {
            render_editor(frame, editor, &self.theme);
            return;
        }
        if let Some(ref viewer) = self.viewer {
            render_viewer(frame, viewer, &self.theme);
            return;
        }
        if let Some(ref help) = self.help {
            render_help(frame, help, &self.theme);
            return;
        }

        if !self.panels_visible {
            let active_dir = match self.active_panel {
                PanelSide::Left => self.left_panel.current_dir.clone(),
                PanelSide::Right => self.right_panel.current_dir.clone(),
            };
            command_line::render_command_line(
                frame,
                size,
                &self.command_line,
                &active_dir,
                &self.theme,
            );
            return;
        }

        // Layout: panels (fills remaining) | command box (3 rows) | fn bar (1 row)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Panels
                Constraint::Length(3), // Command line box
                Constraint::Length(1), // Function key bar
            ])
            .split(size);

        // Split: 50/50 for both tree panels
        let panel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[0]);

        // Scroll adjustments
        let left_height = panel_chunks[0].height.saturating_sub(3) as usize;
        self.left_tree.scroll_to_cursor(left_height);
        let right_height = panel_chunks[1].height.saturating_sub(3) as usize;
        self.right_tree.scroll_to_cursor(right_height);

        // Render left tree panel
        render_tree_panel(
            frame,
            panel_chunks[0],
            &self.left_tree,
            self.active_panel == PanelSide::Left,
            &self.theme,
        );

        // Render right tree panel (or info panel if Ctrl+L toggled)
        if self.show_info_panel {
            let data = InfoPanelData::from_panel(self.active_panel_ref());
            render_info_panel(frame, panel_chunks[1], &data, &self.theme);
        } else {
            render_tree_panel(
                frame,
                panel_chunks[1],
                &self.right_tree,
                self.active_panel == PanelSide::Right,
                &self.theme,
            );
        }

        // Render command line
        let active_dir = match self.active_panel {
            PanelSide::Left => &self.left_panel.current_dir,
            PanelSide::Right => &self.right_panel.current_dir,
        };
        command_line::render_command_line(
            frame,
            main_chunks[1],
            &self.command_line,
            active_dir,
            &self.theme,
        );

        // Render function key bar
        if self.config.ui.show_fn_bar {
            fn_bar::render_fn_bar(frame, main_chunks[2], &self.theme);
        }

        // Render overlays on top: menu > search > AI bar > dialog
        if let Some(ref menu) = self.menu {
            render_menu(frame, menu, &self.theme);
        }

        if let Some(ref search) = self.search {
            render_search(frame, search, &self.theme);
        }

        if let Some(ref ai_bar) = self.ai_bar {
            render_ai_bar(frame, ai_bar, &self.theme);
        }

        if let Some(ref dialog) = self.dialog {
            render_dialog(frame, dialog, &self.theme);
        }
    }
}
