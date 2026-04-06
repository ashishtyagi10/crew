use std::path::{Path, PathBuf};

use crossterm::event::KeyEvent;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use farx_core::{Action, AppConfig, KeyMap, PanelSide, PanelState, TreeState};

use farx_core::SortField;

use crate::components::ai_bar::{render_ai_bar, AiBarAction, AiBarState};
use crate::components::bookmarks::{
    load_bookmarks, render_bookmarks, save_bookmarks, Bookmark, BookmarkAction, BookmarkState,
};
use crate::components::command_line::CommandLineState;
use crate::components::dialog::{render_dialog, DialogResult, DialogState};
use crate::components::editor::{render_editor, EditorAction, EditorState};
use crate::components::feedback::{render_feedback, ConfirmAction, FeedbackResult, FeedbackState};
use crate::components::help::{render_help, HelpState};
use crate::components::info_panel::{render_info_panel, InfoPanelData};
use crate::components::menu::{render_menu, MenuAction, MenuState};
use crate::components::search::{render_search, SearchAction, SearchState};
use crate::components::tree_panel::render_tree_panel_with_filter;
use crate::components::viewer::{render_viewer, ViewerAction, ViewerState};
use crate::components::{command_line, fn_bar};
use crate::theme::Theme;

/// Pending operation for input dialogs (MkDir, Rename, CreateFile).
#[derive(Debug, Clone)]
enum PendingOperation {
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
    /// Inline feedback system (replaces modal dialogs for messages/confirms).
    pub feedback: FeedbackState,
    /// Tick counter for debounce timing.
    tick_count: u64,
    /// Pending typeahead suggestion response.
    suggestion_rx: Option<tokio::sync::oneshot::Receiver<Option<String>>>,
    /// The input text the pending suggestion was requested for.
    suggestion_request_input: String,
    /// Tree view state for the left panel.
    pub left_tree: TreeState,
    /// Tree view state for the right panel.
    pub right_tree: TreeState,
    /// If set, a newer version is available for update.
    pub update_available: Option<String>,
    /// Bookmarks panel state (Ctrl+B).
    pub bookmarks_panel: Option<BookmarkState>,
    /// Persisted bookmarks list.
    pub bookmarks: Vec<Bookmark>,
    /// Filter state for narrowing directory listing.
    pub filter_active: bool,
    /// Current filter pattern.
    pub filter_pattern: String,
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
            feedback: FeedbackState::new(),
            tick_count: 0,
            suggestion_rx: None,
            suggestion_request_input: String::new(),
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
            update_available: None,
            bookmarks_panel: None,
            bookmarks: load_bookmarks(),
            filter_active: false,
            filter_pattern: String::new(),
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

    /// Get the active tree (immutable).
    fn active_tree_ref(&self) -> &TreeState {
        match self.active_panel {
            PanelSide::Left => &self.left_tree,
            PanelSide::Right => &self.right_tree,
        }
    }

    /// Navigate the active panel to a new directory.
    /// Updates both the tree root and the panel's current_dir.
    fn navigate_to(&mut self, path: PathBuf) {
        match self.active_panel {
            PanelSide::Left => {
                self.left_tree.navigate_to(path.clone());
                self.left_panel.current_dir = path;
            }
            PanelSide::Right => {
                self.right_tree.navigate_to(path.clone());
                self.right_panel.current_dir = path;
            }
        }
    }

    /// Get the inactive tree's root directory.
    fn inactive_tree_root(&self) -> PathBuf {
        match self.active_panel {
            PanelSide::Left => self.right_tree.root.clone(),
            PanelSide::Right => self.left_tree.root.clone(),
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

    /// Collect paths from tree selection (or current node).
    fn collect_selected_paths(&self) -> Vec<PathBuf> {
        let tree = self.active_tree_ref();
        if tree.selected.is_empty() {
            if let Some(node) = tree.current_node() {
                return vec![node.entry.path.clone()];
            }
            Vec::new()
        } else {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .map(|n| n.entry.path.clone())
                .collect()
        }
    }

    /// Collect display names from tree selection.
    fn collect_selected_names(&self) -> Vec<String> {
        let tree = self.active_tree_ref();
        if tree.selected.is_empty() {
            if let Some(node) = tree.current_node() {
                return vec![node.entry.name.clone()];
            }
            Vec::new()
        } else {
            tree.selected
                .iter()
                .filter_map(|&i| tree.visible_nodes.get(i))
                .map(|n| n.entry.name.clone())
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
                ViewerAction::Close => {
                    self.viewer = None;
                }
                ViewerAction::None => {}
            }
            return Action::Noop;
        }

        // Inline feedback (confirmations, output panels)
        match self.feedback.handle_key(key) {
            FeedbackResult::Confirmed(_) => {
                if let Some(action) = self.feedback.take_confirm() {
                    self.execute_confirm(action);
                }
                return Action::Noop;
            }
            FeedbackResult::Rejected | FeedbackResult::Consumed => {
                return Action::Noop;
            }
            FeedbackResult::NotHandled => {}
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

        // Bookmarks panel
        if let Some(ref mut bm_panel) = self.bookmarks_panel {
            match bm_panel.handle_key_event(key) {
                BookmarkAction::Close => {
                    self.bookmarks_panel = None;
                }
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
            return Action::Noop;
        }

        // AI bar
        if let Some(ref mut ai_bar) = self.ai_bar {
            match ai_bar.handle_key_event(key) {
                AiBarAction::Close => {
                    self.ai_bar = None;
                }
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

        // Filter mode: intercept key input for filter pattern
        if self.filter_active {
            use crossterm::event::{KeyCode, KeyModifiers};
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => {
                    self.filter_active = false;
                    self.filter_pattern.clear();
                    self.active_tree().filter.clear();
                    self.active_tree().rebuild();
                    return Action::Noop;
                }
                (KeyCode::Enter, _) => {
                    // Accept filter and close filter bar (keep results narrowed)
                    self.filter_active = false;
                    return Action::Noop;
                }
                (KeyCode::Backspace, _) => {
                    self.filter_pattern.pop();
                    self.active_tree().filter = self.filter_pattern.clone();
                    self.active_tree().rebuild();
                    return Action::Noop;
                }
                (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.filter_pattern.push(ch);
                    self.active_tree().filter = self.filter_pattern.clone();
                    self.active_tree().rebuild();
                    return Action::Noop;
                }
                (KeyCode::Up, _) => {
                    self.active_tree().move_cursor(-1);
                    return Action::Noop;
                }
                (KeyCode::Down, _) => {
                    self.active_tree().move_cursor(1);
                    return Action::Noop;
                }
                _ => {
                    return Action::Noop;
                }
            }
        }

        // If command line has input, intercept some keys for command line editing
        if !self.command_line.input.is_empty() {
            use crossterm::event::{KeyCode, KeyModifiers};
            // Tab: accept suggestion if available, otherwise switch panel
            if key.code == KeyCode::Tab && self.command_line.suggestion.is_some() {
                self.command_line.accept_suggestion();
                self.command_line.last_typed_tick = self.tick_count;
                return Action::Noop;
            }
            match (key.code, key.modifiers) {
                (KeyCode::Up, KeyModifiers::NONE) => return Action::CommandLineHistoryUp,
                (KeyCode::Down, KeyModifiers::NONE) => return Action::CommandLineHistoryDown,
                (KeyCode::Esc, _) => return Action::CommandLineClear,
                (KeyCode::Char(' '), KeyModifiers::NONE) => {
                    return Action::CommandLineInput(' ');
                }
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
        let entries: Vec<(String, bool, u64)> = self
            .active_panel_ref()
            .entries
            .iter()
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

    /// Called when the background update check finds a newer version.
    pub fn set_update_available(&mut self, version: String) {
        self.feedback.info(format!(
            "Update available: v{version} — run `farx --update` to install"
        ));
        self.update_available = Some(version);
    }

    pub fn set_update_applied(&mut self, version: String) {
        self.feedback
            .success(format!("Updated to v{version} — restart farx to use it"));
        self.update_available = None;
    }

    /// Check for completed AI responses (called from tick).
    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.feedback.tick();
        self.check_ai_response();
        self.check_suggestion_response();

        // Debounced typeahead: request suggestion after 3 ticks (~750ms) of no typing
        if !self.command_line.input.is_empty()
            && !self.command_line.suggestion_pending
            && self.command_line.suggestion.is_none()
            && self.command_line.last_typed_tick > 0
            && self.tick_count - self.command_line.last_typed_tick >= 3
        {
            self.request_suggestion();
        }
    }

    /// Request a typeahead suggestion from the LLM.
    fn request_suggestion(&mut self) {
        let input = self.command_line.input.clone();
        if input.len() < 2 {
            return; // Don't suggest for very short input
        }
        self.command_line.suggestion_pending = true;
        self.command_line.suggestion_for = input.clone();
        self.suggestion_request_input = input.clone();

        let dir = self.active_tree_ref().root.clone();
        let entries: Vec<(String, bool, u64)> = self
            .active_tree_ref()
            .visible_nodes
            .iter()
            .take(20) // only send a few for speed
            .map(|n| (n.entry.name.clone(), n.entry.is_dir, n.entry.size))
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
        self.suggestion_rx = Some(rx);

        tokio::spawn(async move {
            let result = agent.suggest(&input, &dir, &files_context).await;
            let _ = tx.send(result.unwrap_or(None));
        });
    }

    /// Check for completed suggestion responses.
    fn check_suggestion_response(&mut self) {
        if let Some(ref mut rx) = self.suggestion_rx {
            match rx.try_recv() {
                Ok(suggestion) => {
                    // Only apply if input hasn't changed since request
                    if self.command_line.input == self.suggestion_request_input {
                        self.command_line.suggestion = suggestion;
                    }
                    self.command_line.suggestion_pending = false;
                    self.suggestion_rx = None;
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                    // Still waiting
                }
                Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                    self.command_line.suggestion_pending = false;
                    self.suggestion_rx = None;
                }
            }
        }
    }

    /// Calculate the size of the directory (or selected items) under the cursor.
    fn calculate_dir_size(&mut self) {
        let tree = self.active_tree_ref();

        // If there are selected items, calculate total for selection
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

        // Single item under cursor
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

    fn check_ai_response(&mut self) {
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

        // Slash commands: /exit, /quit, /help, /refresh, /hidden, /sort, /search, /ai, /cd
        if input.starts_with('/') {
            if self.handle_slash_command(&input) {
                return;
            }
            // Unknown slash command — fall through to shell/AI
        }

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
                    if result.lines().count() <= 1 {
                        // Short output — show as inline info
                        if !result.is_empty() {
                            self.feedback.info(result);
                        }
                    } else {
                        // Multi-line — show as scrollable output panel
                        self.feedback.show_output("Output", result);
                    }
                }
                Err(e) => {
                    self.feedback.error(format!("Command: {}", e));
                }
            }
            self.left_tree.rebuild();
            self.right_tree.rebuild();
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

    /// Handle a slash command. Returns true if the command was recognized.
    fn handle_slash_command(&mut self, input: &str) -> bool {
        let trimmed = input.trim();
        let (cmd, args) = match trimmed.split_once(char::is_whitespace) {
            Some((c, a)) => (c, a.trim()),
            None => (trimmed, ""),
        };

        match cmd {
            "/exit" | "/quit" | "/q" => {
                self.running = false;
            }
            "/help" | "/h" => {
                self.help = Some(HelpState::new());
            }
            "/refresh" | "/r" => {
                self.left_tree.rebuild();
                self.right_tree.rebuild();
                self.feedback.info("Refreshed".to_string());
            }
            "/hidden" => {
                self.config.general.show_hidden_files = !self.config.general.show_hidden_files;
                let sh = self.config.general.show_hidden_files;
                self.left_tree.show_hidden = sh;
                self.left_tree.rebuild();
                self.right_tree.show_hidden = sh;
                self.right_tree.rebuild();
                self.feedback.info(format!(
                    "Hidden files: {}",
                    if sh { "shown" } else { "hidden" }
                ));
            }
            "/sort" => {
                let valid = match args {
                    "name" => {
                        self.active_panel_mut().sort_field = SortField::Name;
                        true
                    }
                    "ext" => {
                        self.active_panel_mut().sort_field = SortField::Extension;
                        true
                    }
                    "size" => {
                        self.active_panel_mut().sort_field = SortField::Size;
                        true
                    }
                    "date" => {
                        self.active_panel_mut().sort_field = SortField::Modified;
                        true
                    }
                    _ => {
                        self.feedback
                            .error("Usage: /sort name|ext|size|date".to_string());
                        false
                    }
                };
                if valid {
                    self.active_panel_mut().sort_entries();
                }
            }
            "/search" | "/find" => {
                let dir = self.active_panel_ref().current_dir.clone();
                self.search = Some(SearchState::new(dir));
            }
            "/ai" => {
                self.ai_bar = Some(AiBarState::new());
            }
            "/cd" => {
                if args.is_empty() {
                    // Go home
                    if let Some(home) = dirs::home_dir() {
                        self.navigate_to(home);
                    }
                } else {
                    let path = if args.starts_with('~') {
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(args.trim_start_matches("~/"))
                    } else if args.starts_with('/') {
                        PathBuf::from(args)
                    } else {
                        self.active_tree_ref().root.join(args)
                    };
                    if path.is_dir() {
                        self.navigate_to(path);
                    } else {
                        self.feedback.error(format!("Not a directory: {}", args));
                    }
                }
            }
            "/menu" => {
                self.menu = Some(MenuState::new());
            }
            "/back" => {
                self.dispatch(Action::HistoryBack);
            }
            "/forward" | "/fwd" => {
                self.dispatch(Action::HistoryForward);
            }
            "/size" => {
                self.calculate_dir_size();
            }
            "/filter" => {
                if args.is_empty() {
                    self.filter_active = true;
                    self.filter_pattern.clear();
                } else {
                    self.filter_pattern = args.to_string();
                    self.active_tree().filter = args.to_string();
                    self.active_tree().rebuild();
                    self.feedback.info(format!("Filter: {}", args));
                }
            }
            "/bookmark" | "/bm" => {
                if args.is_empty() {
                    self.bookmarks_panel = Some(BookmarkState::new(self.bookmarks.clone()));
                } else {
                    // /bookmark add — add current dir
                    self.dispatch(Action::AddBookmark);
                }
            }
            "/info" => {
                self.show_info_panel = !self.show_info_panel;
            }
            _ => return false,
        }
        true
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
        if first_word.starts_with('/')
            || first_word.starts_with("./")
            || first_word.starts_with("~/")
        {
            return true;
        }

        // Contains shell operators
        if trimmed.contains('|')
            || trimmed.contains('>')
            || trimmed.contains('<')
            || trimmed.contains("&&")
            || trimmed.contains("||")
            || trimmed.contains(';')
        {
            return true;
        }

        // Starts with common command names
        const SHELL_COMMANDS: &[&str] = &[
            "ls",
            "cd",
            "cp",
            "mv",
            "rm",
            "mkdir",
            "rmdir",
            "cat",
            "head",
            "tail",
            "grep",
            "find",
            "sed",
            "awk",
            "sort",
            "uniq",
            "wc",
            "echo",
            "printf",
            "touch",
            "chmod",
            "chown",
            "chgrp",
            "ln",
            "pwd",
            "env",
            "export",
            "which",
            "whereis",
            "whoami",
            "date",
            "cal",
            "df",
            "du",
            "free",
            "top",
            "ps",
            "kill",
            "tar",
            "zip",
            "unzip",
            "gzip",
            "gunzip",
            "curl",
            "wget",
            "ssh",
            "scp",
            "rsync",
            "git",
            "docker",
            "make",
            "npm",
            "yarn",
            "pnpm",
            "cargo",
            "rustc",
            "python",
            "python3",
            "pip",
            "node",
            "ruby",
            "go",
            "java",
            "javac",
            "gcc",
            "g++",
            "clang",
            "brew",
            "apt",
            "yum",
            "dnf",
            "pacman",
            "snap",
            "flatpak",
            "systemctl",
            "journalctl",
            "sudo",
            "su",
            "man",
            "less",
            "more",
            "vi",
            "vim",
            "nano",
            "emacs",
            "code",
            "open",
            "xdg-open",
            "clear",
            "reset",
            "history",
            "alias",
            "unalias",
            "set",
            "unset",
            "test",
            "true",
            "false",
            "yes",
            "no",
            "tee",
            "xargs",
            "diff",
            "patch",
            "file",
            "stat",
            "md5",
            "sha256sum",
            "base64",
        ];

        if SHELL_COMMANDS.contains(&first_word) {
            return true;
        }

        // Environment variable assignment (FOO=bar)
        if first_word.contains('=') && !first_word.starts_with('=') {
            return true;
        }

        // If first word contains a dot and looks like a script (./foo.sh, script.py)
        if first_word.contains('.')
            && (first_word.ends_with(".sh")
                || first_word.ends_with(".py")
                || first_word.ends_with(".rb")
                || first_word.ends_with(".js")
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

    /// Execute the file operation associated with a confirmed input dialog.
    fn execute_pending_operation(&mut self, op: PendingOperation, input_value: Option<String>) {
        let result = match op {
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

        // Refresh trees after file operation
        self.left_tree.rebuild();
        self.right_tree.rebuild();

        match result {
            Ok(()) => self.feedback.success("Done"),
            Err(e) => self.feedback.error(format!("{e}")),
        }
    }

    /// Show an error dialog.
    fn show_error(&mut self, title: &str, message: &str) {
        self.feedback.error(format!("{}: {}", title, message));
    }

    /// Execute a confirmed file operation.
    fn execute_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::Copy { sources, dest } => {
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
            ConfirmAction::Move { sources, dest } => {
                let mut ok = 0;
                let mut fail = 0;
                for source in &sources {
                    match farx_fs::move_entry(source, &dest) {
                        Ok(()) => ok += 1,
                        Err(_) => fail += 1,
                    }
                }
                if fail == 0 {
                    self.feedback.success(format!("Moved {} file(s)", ok));
                } else {
                    self.feedback
                        .warning(format!("Moved {}, failed {}", ok, fail));
                }
            }
            ConfirmAction::Delete { targets } => {
                let mut ok = 0;
                let mut fail = 0;
                for target in &targets {
                    match farx_fs::delete_entry(target, false) {
                        Ok(()) => ok += 1,
                        Err(_) => fail += 1,
                    }
                }
                if fail == 0 {
                    self.feedback.success(format!("Deleted {} file(s)", ok));
                } else {
                    self.feedback
                        .warning(format!("Deleted {}, failed {}", ok, fail));
                }
            }
        }
        // Refresh both trees
        self.left_tree.rebuild();
        self.right_tree.rebuild();
    }

    /// Execute an action, updating application state accordingly.
    pub fn dispatch(&mut self, action: Action) {
        // Both panels use tree view — route navigation through the active tree
        match &action {
            Action::CursorUp => {
                self.active_tree().move_cursor(-1);
                return;
            }
            Action::CursorDown => {
                self.active_tree().move_cursor(1);
                return;
            }
            Action::CursorPageUp => {
                self.active_tree().move_cursor(-20);
                return;
            }
            Action::CursorPageDown => {
                self.active_tree().move_cursor(20);
                return;
            }
            Action::CursorHome => {
                self.active_tree().move_cursor_to(0);
                return;
            }
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
                if matches!(action, Action::CommandLineEnterOrDir)
                    && !self.command_line.input.is_empty()
                {
                    self.smart_execute_command();
                    return;
                }
                // Read what we need from the tree node first
                let node_info = self
                    .active_tree_ref()
                    .current_node()
                    .map(|n| (n.entry.is_dir, n.entry.path.clone(), n.entry.name.clone()));
                if let Some((is_dir, path, name)) = node_info {
                    if is_dir {
                        // Enter changes into the directory (new root)
                        self.navigate_to(path);
                    } else {
                        // Smart open: text in editor, binary with system app
                        if is_text_file(&path) {
                            match EditorState::open(&path) {
                                Ok(es) => {
                                    self.editor = Some(es);
                                }
                                Err(e) => {
                                    self.show_error("Edit", &format!("{}", e));
                                }
                            }
                        } else {
                            match open::that(&path) {
                                Ok(()) => self.feedback.info(format!("Opened: {}", name)),
                                Err(e) => self.feedback.error(format!("Open: {}", e)),
                            }
                        }
                    }
                }
                return;
            }
            Action::ParentDirectory => {
                // Go up to the parent directory
                let parent = self
                    .active_tree_ref()
                    .root
                    .parent()
                    .map(|p| p.to_path_buf());
                if let Some(parent_path) = parent {
                    self.navigate_to(parent_path);
                }
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
                            Ok(vs) => {
                                self.viewer = Some(vs);
                            }
                            Err(e) => {
                                self.show_error("View", &format!("{}", e));
                            }
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
                            Ok(es) => {
                                self.editor = Some(es);
                            }
                            Err(e) => {
                                self.show_error("Edit", &format!("{}", e));
                            }
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
            Action::QuickSearch(ch) => {
                self.active_panel_mut().enter_quick_search(ch);
            }
            Action::QuickSearchClear => {
                self.active_panel_mut().clear_quick_search();
            }
            Action::CommandLineInput(ch) => {
                self.command_line.input_char(ch);
                self.command_line.last_typed_tick = self.tick_count;
            }
            Action::CommandLineBackspace => {
                self.command_line.last_typed_tick = self.tick_count;
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
            Action::CalculateDirSize => {
                self.calculate_dir_size();
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
        let left_active = self.active_panel == PanelSide::Left;
        let left_filter_editing = left_active && self.filter_active;
        render_tree_panel_with_filter(
            frame,
            panel_chunks[0],
            &self.left_tree,
            left_active,
            &self.theme,
            left_filter_editing,
        );

        // Render right tree panel (or info panel if Ctrl+L toggled)
        if self.show_info_panel {
            let data = InfoPanelData::from_panel(self.active_panel_ref());
            render_info_panel(frame, panel_chunks[1], &data, &self.theme);
        } else {
            let right_active = self.active_panel == PanelSide::Right;
            let right_filter_editing = right_active && self.filter_active;
            render_tree_panel_with_filter(
                frame,
                panel_chunks[1],
                &self.right_tree,
                right_active,
                &self.theme,
                right_filter_editing,
            );
        }

        // Render command line / feedback area
        // Feedback (messages, confirmations) replaces the command line when active
        if self.feedback.has_content() {
            render_feedback(frame, main_chunks[1], &self.feedback);
        } else {
            let active_dir = self.active_tree_ref().root.clone();
            command_line::render_command_line(
                frame,
                main_chunks[1],
                &self.command_line,
                &active_dir,
                &self.theme,
            );
        }

        // Render function key bar
        if self.config.ui.show_fn_bar {
            fn_bar::render_fn_bar(frame, main_chunks[2], &self.theme);
        }

        // Overlays: menu > search > AI bar > dialog (only for text input)
        if let Some(ref menu) = self.menu {
            render_menu(frame, menu, &self.theme);
        }

        if let Some(ref search) = self.search {
            render_search(frame, search, &self.theme);
        }

        if let Some(ref ai_bar) = self.ai_bar {
            render_ai_bar(frame, ai_bar, &self.theme);
        }

        // Bookmarks panel
        if let Some(ref bm_panel) = self.bookmarks_panel {
            render_bookmarks(frame, bm_panel, &self.theme);
        }

        // Dialog only for text input (MkDir, Rename, CreateFile)
        if let Some(ref dialog) = self.dialog {
            render_dialog(frame, dialog, &self.theme);
        }

        // Scrollable output panel (from feedback) renders on top of panels
        if self.feedback.output_visible {
            let output_area = main_chunks[0]; // render over the panel area
            render_feedback(frame, output_area, &self.feedback);
        }
    }
}

/// Determine if a file should be opened in the built-in editor (text)
/// or with the system default application (binary/media).
/// Recursively calculate the total size of a directory.
fn dir_size_recursive(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    total += dir_size_recursive(&entry.path());
                } else {
                    total += meta.len();
                }
            }
        }
    }
    total
}

/// Format a byte count into a human-readable size string.
fn format_size_human(size: u64) -> String {
    if size < 1_000 {
        format!("{} B", size)
    } else if size < 1_000_000 {
        format!("{:.1} KB", size as f64 / 1_024.0)
    } else if size < 1_000_000_000 {
        format!("{:.1} MB", size as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", size as f64 / 1_073_741_824.0)
    }
}

fn is_text_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        // Definitely text — open in editor
        Some(
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "h" | "cpp" | "cc"
            | "hpp" | "java" | "kt" | "swift" | "rb" | "pl" | "pm" | "lua" | "php"
            | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd"
            | "html" | "htm" | "css" | "scss" | "less" | "sass"
            | "xml" | "svg" | "json" | "jsonc" | "yaml" | "yml" | "toml" | "ini" | "cfg"
            | "conf" | "env" | "properties"
            | "md" | "markdown" | "txt" | "text" | "log" | "csv" | "tsv"
            | "sql" | "graphql" | "gql"
            | "dockerfile" | "makefile" | "cmake"
            | "gitignore" | "gitattributes" | "editorconfig"
            | "lock" | "sum"  // Cargo.lock, go.sum etc
            | "r" | "R" | "jl" | "ex" | "exs" | "erl" | "hrl" | "elm"
            | "zig" | "nim" | "v" | "d" | "pas" | "pp"
            | "tf" | "hcl" | "nix" | "dhall"
            | "proto" | "thrift" | "avsc"
            | "vue" | "svelte" | "astro"
        ) => true,

        // Definitely binary — open with system app
        Some(
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
            | "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "webp"
            | "heic" | "heif" | "raw" | "cr2" | "nef" | "svg"  // svg as image
            | "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a"
            | "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v"
            | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "lz"
            | "dmg" | "iso" | "img" | "pkg" | "deb" | "rpm" | "msi" | "exe" | "app"
            | "so" | "dylib" | "dll" | "a" | "lib" | "o" | "obj"
            | "class" | "jar" | "war" | "pyc" | "pyo" | "wasm"
            | "ttf" | "otf" | "woff" | "woff2" | "eot"
            | "db" | "sqlite" | "sqlite3"
            | "psd" | "ai" | "sketch" | "fig" | "xd"
        ) => false,

        // No extension — try to detect by reading first bytes
        None => {
            // Check if the filename itself is a known text file
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            matches!(name.to_lowercase().as_str(),
                "makefile" | "dockerfile" | "vagrantfile" | "gemfile" | "rakefile"
                | "procfile" | "brewfile" | "justfile" | "taskfile"
                | ".gitignore" | ".gitattributes" | ".editorconfig" | ".env"
                | ".bashrc" | ".zshrc" | ".profile" | ".vimrc"
                | "license" | "readme" | "changelog" | "authors" | "todo"
            ) || {
                // Heuristic: try reading first 512 bytes, check for null bytes
                std::fs::read(path)
                    .map(|bytes| {
                        let check = &bytes[..bytes.len().min(512)];
                        !check.contains(&0) // no null bytes = likely text
                    })
                    .unwrap_or(false)
            }
        }

        // Unknown extension — try binary detection
        Some(_) => {
            std::fs::read(path)
                .map(|bytes| {
                    let check = &bytes[..bytes.len().min(512)];
                    !check.contains(&0)
                })
                .unwrap_or(false)
        }
    }
}
