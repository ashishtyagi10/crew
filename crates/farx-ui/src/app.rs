use std::path::{Path, PathBuf};

use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use farx_core::{Action, AppConfig, KeyMap, PanelSide, PanelState, SortOrder, TabGroup, TreeState};

use farx_core::SortField;

use crate::components::ai_bar::{render_ai_bar, AiBarAction, AiBarState};
use crate::components::ai_panel::{render_ai_panel, AiPanelAction, AiPanelState};
use crate::components::batch_rename::{render_batch_rename, BatchRenameAction, BatchRenameState};
use crate::components::bookmarks::{
    load_bookmarks, render_bookmarks, save_bookmarks, Bookmark, BookmarkAction, BookmarkState,
};
use crate::components::chmod_dialog::{render_chmod_dialog, ChmodAction, ChmodDialogState};
use crate::components::command_line::CommandLineState;
use crate::components::dialog::{render_dialog, DialogResult, DialogState};
use crate::components::diff_view::{render_diff_view, DiffAction, DiffViewState};
use crate::components::editor::{render_editor, EditorAction, EditorState};
use crate::components::embedded_terminal::{key_to_bytes, render_terminal, TerminalSession};
use crate::components::feedback::{render_feedback, ConfirmAction, FeedbackResult, FeedbackState};
use crate::components::fuzzy_finder::{render_fuzzy_finder, FuzzyAction, FuzzyFinderState};
use crate::components::help::{render_help, HelpState};
use crate::components::info_panel::{render_info_panel, InfoPanelData};
use crate::components::menu::{render_menu, MenuAction, MenuState};
use crate::components::progress::{render_progress, ProgressState};
use crate::components::quick_actions::{
    render_quick_actions, QuickActionResult, QuickActionsState,
};
use crate::components::search::{render_search, SearchAction, SearchState};
use crate::components::slash_suggestions::{render_slash_suggestions, SlashSuggestionsState};
use crate::components::tree_panel::{render_tab_bar, render_tree_panel_with_filter};
use crate::components::viewer::{render_viewer, ViewerAction, ViewerState};
use crate::components::{command_line, fn_bar};
use crate::theme::Theme;

/// Pending operation for input dialogs (MkDir, Rename, CreateFile).
#[derive(Debug, Clone)]
enum PendingOperation {
    MkDir { parent: PathBuf },
    Rename { original: PathBuf },
    CreateFile { parent: PathBuf },
    CopySameDir { source: PathBuf },
    SelectByMask,
    DeselectByMask,
    CreateSymlink { target: PathBuf },
    GotoDirectory,
}

/// A recorded file operation that can be undone.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum UndoEntry {
    /// Files were deleted (moved to trash). Record paths for feedback.
    Delete { paths: Vec<PathBuf> },
    /// Files were moved from sources to dest dir.
    Move {
        sources: Vec<PathBuf>,
        dest: PathBuf,
    },
    /// A file was renamed from old to new.
    Rename { old: PathBuf, new: PathBuf },
    /// A directory was created.
    MkDir { path: PathBuf },
    /// A file was created.
    CreateFile { path: PathBuf },
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
    /// Tree view state for the left panel (supports multiple tabs).
    pub left_tree: TabGroup,
    /// Tree view state for the right panel (supports multiple tabs).
    pub right_tree: TabGroup,
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
    /// Plugin engine for Lua extensions.
    pub plugin_engine: Option<farx_plugin::PluginEngine>,
    /// Undo stack for file operations.
    undo_stack: Vec<UndoEntry>,
    /// Batch rename dialog state.
    pub batch_rename: Option<BatchRenameState>,
    /// Chmod / permissions dialog state.
    pub chmod_dialog: Option<ChmodDialogState>,
    /// File operation progress dialog.
    pub progress: Option<ProgressState>,
    /// Side-by-side diff view state.
    pub diff_view: Option<DiffViewState>,
    /// Fuzzy finder dialog state.
    pub fuzzy_finder: Option<FuzzyFinderState>,
    /// Quick actions palette state.
    pub quick_actions: Option<QuickActionsState>,
    /// AI tools selector panel state.
    pub ai_panel: Option<AiPanelState>,
    /// Slash command suggestion popup state.
    pub slash_suggestions: Option<SlashSuggestionsState>,
    /// Embedded terminal sessions.
    pub terminals: Vec<crate::components::embedded_terminal::TerminalSession>,
    /// Panel layout tree (supports recursive splitting).
    pub layout: farx_core::LayoutNode,
    /// Index of the focused leaf in the layout tree (None = file panel via active_panel).
    pub focused_terminal: Option<usize>,
    /// File watcher: receives notifications when files change.
    fs_watcher: Option<notify::RecommendedWatcher>,
    fs_change_rx: Option<std::sync::mpsc::Receiver<()>>,
    /// Debounce: tick count of last fs change signal.
    fs_change_tick: u64,
    /// Cached panel rects from last render (for mouse hit-testing).
    cached_panel_rects: Vec<(farx_core::PanelLeaf, ratatui::layout::Rect)>,
    /// Cached fn-bar rect from last render (for mouse hit-testing).
    cached_fn_bar_rect: Option<ratatui::layout::Rect>,
    /// Last mouse click position and tick for double-click detection.
    last_click: Option<(u16, u16, u64)>,
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

        let mut app = Self {
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
                TabGroup::new(t)
            },
            right_tree: {
                let mut t = TreeState::new(home2);
                t.show_hidden = show_hidden;
                TabGroup::new(t)
            },
            update_available: None,
            bookmarks_panel: None,
            bookmarks: load_bookmarks(),
            filter_active: false,
            filter_pattern: String::new(),
            plugin_engine: {
                match farx_plugin::PluginEngine::new() {
                    Ok(mut engine) => {
                        let _ = engine.load_plugins();
                        Some(engine)
                    }
                    Err(_) => None,
                }
            },
            undo_stack: Vec::new(),
            batch_rename: None,
            chmod_dialog: None,
            progress: None,
            diff_view: None,
            fuzzy_finder: None,
            quick_actions: None,
            ai_panel: None,
            slash_suggestions: None,
            terminals: Vec::new(),
            layout: farx_core::LayoutNode::default_layout(),
            focused_terminal: None,
            fs_watcher: None,
            fs_change_rx: None,
            fs_change_tick: 0,
            cached_panel_rects: Vec::new(),
            cached_fn_bar_rect: None,
            last_click: None,
        };

        app.setup_fs_watcher();
        Ok(app)
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

    /// Refresh all panels (legacy + tree). Called after returning from an
    /// external process that may have modified files.
    pub fn refresh_all(&mut self) {
        self.refresh_both_panels();
        self.left_tree.rebuild();
        self.right_tree.rebuild();
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
    pub fn active_tree_ref(&self) -> &TreeState {
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
        self.update_fs_watcher();
    }

    /// Spawn an embedded terminal in a new split panel.
    fn spawn_embedded_terminal(&mut self, cmd: &str, args: &[&str]) {
        let dir = self.active_tree_ref().root.clone();
        // Use a reasonable default size; will be resized on first render
        let rows = 24;
        let cols = 80;

        match TerminalSession::spawn(cmd, args, &dir, rows, cols) {
            Ok(session) => {
                let terminal_id = self.terminals.len();
                let title = session.title.clone();
                self.terminals.push(session);

                // Find the currently focused leaf index and split it
                let leaves = self.layout.leaves();
                let focus_idx = if let Some(tid) = self.focused_terminal {
                    // Find the leaf that is Terminal(tid)
                    leaves
                        .iter()
                        .position(|l| *l == farx_core::PanelLeaf::Terminal(tid))
                        .unwrap_or(0)
                } else {
                    // Find the leaf that is FilePanel(active_panel)
                    leaves
                        .iter()
                        .position(|l| *l == farx_core::PanelLeaf::FilePanel(self.active_panel))
                        .unwrap_or(0)
                };

                self.layout.split_leaf(focus_idx, terminal_id);

                // Focus the new terminal
                let new_leaves = self.layout.leaves();
                if let Some(idx) = new_leaves
                    .iter()
                    .position(|l| *l == farx_core::PanelLeaf::Terminal(terminal_id))
                {
                    // Set focus to the new terminal
                    self.focused_terminal = Some(terminal_id);
                    let _ = idx; // used for position finding
                }

                self.feedback
                    .info(format!("{} opened in split panel", title));
            }
            Err(e) => {
                self.feedback
                    .error(format!("Failed to spawn terminal: {}", e));
            }
        }
    }

    /// Close a terminal session and collapse its split.
    fn close_terminal(&mut self, terminal_id: usize) {
        if terminal_id >= self.terminals.len() {
            return;
        }

        // Remove from layout tree
        self.layout.remove_terminal(terminal_id);

        // Remove from terminals vec
        self.terminals.remove(terminal_id);

        // Adjust IDs in the layout tree (shift down IDs above the removed one)
        self.layout.adjust_terminal_ids(terminal_id);

        // Also adjust focused_terminal
        match self.focused_terminal {
            Some(id) if id == terminal_id => {
                // Focus was on the closed terminal, go back to file panel
                self.focused_terminal = None;
            }
            Some(id) if id > terminal_id => {
                self.focused_terminal = Some(id - 1);
            }
            _ => {}
        }
    }

    /// Cycle focus to the next panel (file or terminal) — Tab key.
    fn cycle_focus(&mut self) {
        let leaves = self.layout.leaves();
        if leaves.is_empty() {
            return;
        }

        // Find current focus position in leaf list
        let current_idx = if let Some(tid) = self.focused_terminal {
            leaves
                .iter()
                .position(|l| *l == farx_core::PanelLeaf::Terminal(tid))
                .unwrap_or(0)
        } else {
            leaves
                .iter()
                .position(|l| *l == farx_core::PanelLeaf::FilePanel(self.active_panel))
                .unwrap_or(0)
        };

        // Move to next leaf
        let next_idx = (current_idx + 1) % leaves.len();
        match leaves[next_idx] {
            farx_core::PanelLeaf::FilePanel(side) => {
                self.focused_terminal = None;
                self.active_panel = side;
            }
            farx_core::PanelLeaf::Terminal(tid) => {
                self.focused_terminal = Some(tid);
                // Clear attention flag when focusing
                if let Some(t) = self.terminals.get_mut(tid) {
                    t.has_attention = false;
                }
            }
        }
    }

    /// Poll all terminal sessions for new output. Called on each tick.
    pub fn poll_terminals(&mut self) {
        let focused_tid = self.focused_terminal;
        for (i, term) in self.terminals.iter_mut().enumerate() {
            let got_output = term.poll_output();
            // Set attention if terminal has output and isn't focused
            if got_output && Some(i) != focused_tid {
                term.has_attention = true;
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

        // Diff view is full-screen
        if let Some(ref mut diff) = self.diff_view {
            match diff.handle_key_event(key) {
                DiffAction::Close => {
                    self.diff_view = None;
                }
                DiffAction::None => {}
            }
            return Action::Noop;
        }

        // Embedded terminal — when focused, forward all keys to PTY
        // F4 = cycle panels (escape hatch), Ctrl+W = close terminal
        if let Some(tid) = self.focused_terminal {
            use crossterm::event::{KeyCode, KeyModifiers};
            if key.code == KeyCode::F(4) {
                // F4: cycle focus to next panel
                self.cycle_focus();
                return Action::Noop;
            }
            if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl+W: close this terminal
                self.close_terminal(tid);
                return Action::Noop;
            }
            // Forward everything else to the PTY
            if let Some(bytes) = key_to_bytes(&key) {
                if let Some(term) = self.terminals.get_mut(tid) {
                    if term.alive {
                        term.write_input(&bytes);
                    } else {
                        // Process exited — any key closes the panel
                        self.close_terminal(tid);
                    }
                }
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

        // Fuzzy finder
        if let Some(ref mut ff) = self.fuzzy_finder {
            match ff.handle_key_event(key) {
                FuzzyAction::Close => {
                    self.fuzzy_finder = None;
                }
                FuzzyAction::GoTo(path) => {
                    self.fuzzy_finder = None;
                    if path.is_dir() {
                        self.navigate_to(path);
                    } else if let Some(parent) = path.parent() {
                        self.navigate_to(parent.to_path_buf());
                    }
                }
                FuzzyAction::None => {}
            }
            return Action::Noop;
        }

        // AI tools panel
        if let Some(ref mut ai_panel) = self.ai_panel {
            match ai_panel.handle_key_event(key) {
                AiPanelAction::Close => {
                    self.ai_panel = None;
                }
                AiPanelAction::Launch(tool) => {
                    self.ai_panel = None;
                    let (cmd, args) = tool.command();
                    self.spawn_embedded_terminal(cmd, args);
                }
                AiPanelAction::None => {}
            }
            return Action::Noop;
        }

        // Quick actions palette
        if let Some(ref mut qa) = self.quick_actions {
            match qa.handle_key_event(key) {
                QuickActionResult::Close => {
                    self.quick_actions = None;
                }
                QuickActionResult::Execute(cmd) => {
                    self.quick_actions = None;
                    self.handle_quick_action(&cmd);
                }
                QuickActionResult::None => {}
            }
            return Action::Noop;
        }

        // Batch rename dialog
        if let Some(ref mut br) = self.batch_rename {
            match br.handle_key_event(key) {
                BatchRenameAction::Close => {
                    self.batch_rename = None;
                }
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
            return Action::Noop;
        }

        // Chmod dialog
        if let Some(ref mut chmod) = self.chmod_dialog {
            match chmod.handle_key_event(key) {
                ChmodAction::Cancel => {
                    self.chmod_dialog = None;
                }
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
                }
                ChmodAction::None => {}
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

            // Slash command suggestion navigation takes priority
            if self.slash_suggestions.is_some() {
                match key.code {
                    KeyCode::Up => {
                        if let Some(ref mut ss) = self.slash_suggestions {
                            ss.move_up();
                        }
                        return Action::Noop;
                    }
                    KeyCode::Down => {
                        if let Some(ref mut ss) = self.slash_suggestions {
                            ss.move_down();
                        }
                        return Action::Noop;
                    }
                    KeyCode::Tab => {
                        // Tab fills the command and lets user add arguments
                        if let Some(ref ss) = self.slash_suggestions {
                            if let Some(cmd) = ss.selected_command() {
                                self.command_line.input = cmd.to_string();
                                self.command_line.cursor_pos = self.command_line.input.len();
                                self.command_line.input.push(' ');
                                self.command_line.cursor_pos += 1;
                            }
                        }
                        self.slash_suggestions = None;
                        return Action::Noop;
                    }
                    KeyCode::Enter => {
                        // Enter selects and immediately executes the command
                        if let Some(ref ss) = self.slash_suggestions {
                            if let Some(cmd) = ss.selected_command() {
                                self.command_line.input = cmd.to_string();
                                self.command_line.cursor_pos = self.command_line.input.len();
                            }
                        }
                        self.slash_suggestions = None;
                        self.smart_execute_command();
                        return Action::Noop;
                    }
                    KeyCode::Esc => {
                        self.slash_suggestions = None;
                        self.command_line.clear();
                        return Action::Noop;
                    }
                    _ => {
                        // Fall through to normal input handling
                    }
                }
            }

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

    /// Handle mouse events (scroll, click, double-click).
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        let (mx, my) = (mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                // Route scroll to active full-screen overlay first
                if let Some(ref mut editor) = self.editor {
                    let amount: i32 = if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                        -3
                    } else {
                        3
                    };
                    editor.scroll_offset = if amount < 0 {
                        editor.scroll_offset.saturating_sub((-amount) as usize)
                    } else {
                        (editor.scroll_offset + amount as usize)
                            .min(editor.lines.len().saturating_sub(1))
                    };
                    return;
                }
                if let Some(ref mut viewer) = self.viewer {
                    viewer.handle_mouse_event(mouse);
                    return;
                }
                // Find which panel the mouse is over and scroll it
                if let Some(side) = self.panel_side_at(mx, my) {
                    let tree = match side {
                        PanelSide::Left => &mut self.left_tree,
                        PanelSide::Right => &mut self.right_tree,
                    };
                    match mouse.kind {
                        MouseEventKind::ScrollUp => tree.move_cursor(-3),
                        MouseEventKind::ScrollDown => tree.move_cursor(3),
                        _ => {}
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Ignore clicks when full-screen overlays are active
                if self.editor.is_some() || self.viewer.is_some() || self.help.is_some() {
                    return;
                }
                // Ignore clicks when modal dialogs are active
                if self.dialog.is_some()
                    || self.menu.is_some()
                    || self.search.is_some()
                    || self.ai_bar.is_some()
                    || self.bookmarks_panel.is_some()
                    || self.fuzzy_finder.is_some()
                    || self.quick_actions.is_some()
                    || self.batch_rename.is_some()
                    || self.chmod_dialog.is_some()
                {
                    return;
                }

                // Check fn-bar click
                if let Some(fn_rect) = self.cached_fn_bar_rect {
                    if my >= fn_rect.y
                        && my < fn_rect.y + fn_rect.height
                        && mx >= fn_rect.x
                        && mx < fn_rect.x + fn_rect.width
                    {
                        self.handle_fn_bar_click(mx, fn_rect);
                        return;
                    }
                }

                // Check breadcrumb click (top border row of a panel)
                if let Some(path) = self.breadcrumb_hit(mx, my) {
                    if path.is_dir() {
                        let show_hidden = self.config.general.show_hidden_files;
                        let tree = self.active_tree();
                        tree.set_root(path);
                        tree.show_hidden = show_hidden;
                        tree.rebuild();
                    }
                    return;
                }

                // Check panel click
                if let Some((side, row_in_list)) = self.panel_row_at(mx, my) {
                    // Switch active panel if needed
                    if self.active_panel != side {
                        self.active_panel = side;
                        // Sync tree root
                        let tree = match side {
                            PanelSide::Left => &mut self.left_tree,
                            PanelSide::Right => &mut self.right_tree,
                        };
                        let _ = tree;
                    }

                    // Move cursor to the clicked row
                    if let Some(row) = row_in_list {
                        let tree = match side {
                            PanelSide::Left => &mut self.left_tree,
                            PanelSide::Right => &mut self.right_tree,
                        };
                        let target = tree.scroll_offset + row;
                        if target < tree.visible_nodes.len() {
                            tree.cursor = target;
                        }
                    }

                    // Double-click detection (within 8 ticks ≈ 2 seconds)
                    let is_double = if let Some((lx, ly, lt)) = self.last_click {
                        lx == mx && ly == my && self.tick_count.saturating_sub(lt) < 8
                    } else {
                        false
                    };

                    if is_double {
                        self.last_click = None;
                        self.dispatch(Action::EnterDirectory);
                    } else {
                        self.last_click = Some((mx, my, self.tick_count));
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                // Right-click on a panel entry → toggle selection
                if self.editor.is_some() || self.viewer.is_some() || self.help.is_some() {
                    return;
                }
                if let Some((side, row_in_list)) = self.panel_row_at(mx, my) {
                    if self.active_panel != side {
                        self.active_panel = side;
                    }
                    if let Some(row) = row_in_list {
                        let tree = match side {
                            PanelSide::Left => &mut self.left_tree,
                            PanelSide::Right => &mut self.right_tree,
                        };
                        let target = tree.scroll_offset + row;
                        if target < tree.visible_nodes.len() {
                            tree.cursor = target;
                            if tree.selected.contains(&target) {
                                tree.selected.remove(&target);
                            } else {
                                tree.selected.insert(target);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Determine which panel side a screen coordinate falls in.
    fn panel_side_at(&self, x: u16, y: u16) -> Option<PanelSide> {
        for (leaf, rect) in &self.cached_panel_rects {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                if let farx_core::PanelLeaf::FilePanel(side) = leaf {
                    return Some(*side);
                }
            }
        }
        None
    }

    /// Determine panel side and the row index within the file list area.
    /// Returns (side, Some(row_in_visible_list)) or (side, None) if click is on header/footer.
    fn panel_row_at(&self, x: u16, y: u16) -> Option<(PanelSide, Option<usize>)> {
        for (leaf, rect) in &self.cached_panel_rects {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                if let farx_core::PanelLeaf::FilePanel(side) = leaf {
                    // Inner area: 1 for top border
                    let inner_y = rect.y + 1;
                    // Filter bar may take 1 row at the top of inner
                    let tree = match side {
                        PanelSide::Left => &self.left_tree,
                        PanelSide::Right => &self.right_tree,
                    };
                    let filter_height: u16 = if !tree.filter.is_empty() || self.filter_active {
                        1
                    } else {
                        0
                    };
                    let list_start_y = inner_y + filter_height;
                    // Footer is 1 row, bottom border is 1 row
                    let list_end_y = rect.y + rect.height - 2; // -1 border -1 footer
                    if y >= list_start_y && y < list_end_y {
                        let row = (y - list_start_y) as usize;
                        return Some((*side, Some(row)));
                    }
                    return Some((*side, None));
                }
            }
        }
        None
    }

    /// Check if a click hit a breadcrumb segment in the panel title bar.
    fn breadcrumb_hit(&self, x: u16, y: u16) -> Option<std::path::PathBuf> {
        use crate::components::tree_panel::breadcrumb_path_at_click;
        for (leaf, rect) in &self.cached_panel_rects {
            // Top border row of the panel
            if y == rect.y && x >= rect.x && x < rect.x + rect.width {
                if let farx_core::PanelLeaf::FilePanel(side) = leaf {
                    let tree = match side {
                        PanelSide::Left => &self.left_tree,
                        PanelSide::Right => &self.right_tree,
                    };
                    // Switch to this panel if needed
                    return breadcrumb_path_at_click(&tree.root, *rect, x);
                }
            }
        }
        None
    }

    /// Handle a click on the function key bar.
    fn handle_fn_bar_click(&mut self, mx: u16, fn_rect: ratatui::layout::Rect) {
        let total_width = fn_rect.width as usize;
        let item_count = 10usize; // F1..F10
        let slot_width = if item_count > 0 {
            total_width / item_count
        } else {
            return;
        };
        let click_offset = (mx - fn_rect.x) as usize;
        let slot_index = click_offset / slot_width;
        let action = match slot_index {
            0 => Action::ShowHelp,      // F1
            1 => Action::OpenSystemApp, // F2
            2 => Action::EditFile,      // F3
            3 => Action::SwitchPanel,   // F4
            4 => Action::CopyDialog,    // F5
            5 => Action::MoveDialog,    // F6
            6 => Action::MkDirDialog,   // F7
            7 => Action::DeleteDialog,  // F8
            8 => Action::ShowMenu,      // F9
            9 => Action::Quit,          // F10
            _ => return,
        };
        self.dispatch(action);
    }

    fn handle_menu_action(&mut self, action: MenuAction) {
        let show_hidden = self.config.general.show_hidden_files;
        match action {
            MenuAction::SortByName => {
                self.toggle_sort(SortField::Name);
            }
            MenuAction::SortByExtension => {
                self.toggle_sort(SortField::Extension);
            }
            MenuAction::SortBySize => {
                self.toggle_sort(SortField::Size);
            }
            MenuAction::SortByDate => {
                self.toggle_sort(SortField::Modified);
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
            MenuAction::ShowAiPanel => self.dispatch(Action::ShowAiPanel),
            MenuAction::SwapPanels => {
                self.dispatch(Action::SwapPanels);
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
        // Silently store — don't show a feedback popup.
        // The version is shown in the title bar / footer if needed.
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
        self.check_fs_changes();
        self.poll_terminals();

        // Poll file operation progress
        if let Some(ref mut progress) = self.progress {
            if progress.poll() {
                let error = progress.error.clone();
                let files_done = progress.files_done;
                let op = progress.operation.clone();
                self.progress = None;
                if let Some(err) = error {
                    self.feedback.error(format!("{} failed: {}", op, err));
                } else {
                    self.feedback
                        .success(format!("{} complete: {} file(s)", op, files_done));
                }
                self.refresh_both_panels();
            }
        }

        // Viewer follow/tail mode: reload file every 4 ticks (~1s)
        if self.tick_count % 4 == 0 {
            if let Some(ref mut viewer) = self.viewer {
                viewer.reload_if_follow();
            }
        }

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

    /// Select or deselect files matching a glob pattern in the active tree.
    fn apply_mask_selection(&mut self, pattern: &str, select: bool) {
        // Convert simple glob pattern to a match function
        // Supports: * (any chars), ? (single char), and literal matching
        let pat = pattern.to_lowercase();
        let tree = self.active_tree();
        let mut count = 0usize;
        for i in 0..tree.visible_nodes.len() {
            let name = &tree.visible_nodes[i].entry.name;
            if name == ".." {
                continue;
            }
            if glob_match(&pat, &name.to_lowercase()) {
                if select {
                    if tree.selected.insert(i) {
                        count += 1;
                    }
                } else if tree.selected.remove(&i) {
                    count += 1;
                }
            }
        }
        let verb = if select { "Selected" } else { "Deselected" };
        self.feedback
            .info(format!("{} {} file(s) matching '{}'", verb, count, pattern));
    }

    /// Toggle sort: if already sorted by this field, flip asc/desc; otherwise set field and reset to ascending.
    fn toggle_sort(&mut self, field: SortField) {
        let panel = self.active_panel_mut();
        if panel.sort_field == field {
            panel.sort_order = match panel.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            panel.sort_field = field;
            panel.sort_order = SortOrder::Ascending;
        }
        let new_order = panel.sort_order;
        panel.sort_entries();

        // Sync tree sort settings and rebuild so the display actually changes
        let tree = self.active_tree();
        tree.sort_field = field;
        tree.sort_order = new_order;
        tree.rebuild();

        let field_name = match field {
            SortField::Name => "Name",
            SortField::Extension => "Extension",
            SortField::Size => "Size",
            SortField::Modified => "Date",
        };
        let order = match new_order {
            SortOrder::Ascending => "↑",
            SortOrder::Descending => "↓",
        };
        self.feedback
            .info(format!("Sort: {} {}", field_name, order));
    }

    /// Compare directories: select files in both panels that are unique or different.
    fn compare_directories(&mut self) {
        use std::collections::HashMap;

        // Collect other panel data first (owned) to avoid borrow conflict
        let other_tree = match self.active_panel {
            PanelSide::Left => &self.right_tree,
            PanelSide::Right => &self.left_tree,
        };
        let other_files: HashMap<String, (u64, Option<chrono::DateTime<chrono::Local>>)> =
            other_tree
                .visible_nodes
                .iter()
                .filter(|n| n.depth == 0 && n.entry.name != "..")
                .map(|n| (n.entry.name.clone(), (n.entry.size, n.entry.modified)))
                .collect();

        // Select files in the active panel that are unique or differ
        let tree = self.active_tree();
        tree.selected.clear();
        let mut selected_count = 0usize;
        for i in 0..tree.visible_nodes.len() {
            let node = &tree.visible_nodes[i];
            if node.depth != 0 || node.entry.name == ".." {
                continue;
            }
            match other_files.get(&node.entry.name) {
                None => {
                    // Unique to this panel
                    tree.selected.insert(i);
                    selected_count += 1;
                }
                Some(&(other_size, other_modified)) => {
                    // Exists in both — compare size and modified time
                    if node.entry.size != other_size || node.entry.modified != other_modified {
                        tree.selected.insert(i);
                        selected_count += 1;
                    }
                }
            }
        }
        self.feedback.info(format!(
            "Compare: {} file(s) differ or are unique",
            selected_count,
        ));
    }

    /// Show a text-based treemap of disk usage for the current directory.
    fn show_treemap(&mut self) {
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

        // Text-based bar chart
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

    /// Set up filesystem watcher for both panel directories.
    /// Handle a quick action command (may be a special builtin or shell command).
    fn handle_quick_action(&mut self, cmd: &str) {
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

    fn setup_fs_watcher(&mut self) {
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
    fn update_fs_watcher(&mut self) {
        use notify::{RecursiveMode, Watcher};
        if let Some(ref mut w) = self.fs_watcher {
            // Unwatch all, then re-watch current roots
            let _ = w.unwatch(&self.left_tree.root);
            let _ = w.unwatch(&self.right_tree.root);
            let _ = w.watch(&self.left_tree.root, RecursiveMode::NonRecursive);
            let _ = w.watch(&self.right_tree.root, RecursiveMode::NonRecursive);
        }
    }

    /// Check for filesystem change notifications (debounced).
    fn check_fs_changes(&mut self) {
        if let Some(ref rx) = self.fs_change_rx {
            let mut changed = false;
            // Drain all pending notifications
            while rx.try_recv().is_ok() {
                changed = true;
            }
            if changed {
                // Debounce: only rebuild if 2+ ticks since last change
                if self.tick_count - self.fs_change_tick >= 2 {
                    self.left_tree.rebuild();
                    self.right_tree.rebuild();
                }
                self.fs_change_tick = self.tick_count;
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

        // Intercept `cd` as a built-in: change the active panel's directory
        // (like FAR Manager — `cd` in the command line navigates the panel)
        if let Some(cd_arg) = Self::parse_cd_command(&input) {
            let base = self.active_panel_ref().current_dir.clone();
            let path = if cd_arg.is_empty() {
                dirs::home_dir().unwrap_or(base)
            } else if cd_arg == "-" {
                // cd - : go to previous directory from history
                let history = match self.active_panel {
                    PanelSide::Left => &self.left_tree.history_back,
                    PanelSide::Right => &self.right_tree.history_back,
                };
                if let Some(prev) = history.last() {
                    prev.clone()
                } else {
                    self.feedback.error("No previous directory".to_string());
                    return;
                }
            } else if cd_arg.starts_with('~') {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(cd_arg.trim_start_matches("~/").trim_start_matches('~'))
            } else if cd_arg.starts_with('/') {
                PathBuf::from(&cd_arg)
            } else {
                base.join(&cd_arg)
            };
            // Canonicalize to resolve `.`, `..`, and symlinks
            match path.canonicalize() {
                Ok(resolved) if resolved.is_dir() => {
                    self.navigate_to(resolved);
                }
                Ok(resolved) => {
                    self.feedback
                        .error(format!("Not a directory: {}", resolved.display()));
                }
                Err(_) => {
                    self.feedback
                        .error(format!("No such directory: {}", cd_arg));
                }
            }
            return;
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
            "/sort" => match args {
                "name" => self.toggle_sort(SortField::Name),
                "ext" => self.toggle_sort(SortField::Extension),
                "size" => self.toggle_sort(SortField::Size),
                "date" => self.toggle_sort(SortField::Modified),
                _ => {
                    self.feedback
                        .error("Usage: /sort name|ext|size|date".to_string());
                }
            },
            "/search" | "/find" => {
                let dir = self.active_panel_ref().current_dir.clone();
                self.search = Some(SearchState::new(dir));
            }
            "/ai" => {
                self.ai_bar = Some(AiBarState::new());
            }
            "/ai-tools" | "/ait" => {
                self.ai_panel = Some(AiPanelState::new());
            }
            "/claude" => {
                self.spawn_embedded_terminal("claude", &[]);
            }
            "/codex" => {
                self.spawn_embedded_terminal("codex", &[]);
            }
            "/copilot" => {
                self.spawn_embedded_terminal("gh", &["copilot"]);
            }
            "/gemini" => {
                self.spawn_embedded_terminal("gemini", &[]);
            }
            "/opencode" => {
                self.spawn_embedded_terminal("opencode", &[]);
            }
            "/shell" | "/sh" | "/bash" | "/zsh" => {
                let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                self.spawn_embedded_terminal(&shell, &[]);
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
            "/yank" | "/copy-path" => {
                self.dispatch(Action::CopyPathToClipboard);
            }
            "/yank-names" | "/copy-names" => {
                self.dispatch(Action::CopyNameToClipboard);
            }
            "/checksum" | "/sha256" => {
                self.dispatch(Action::ShowChecksums);
            }
            "/chmod" | "/permissions" | "/perms" => {
                self.dispatch(Action::ChmodDialog);
            }
            "/actions" => {
                self.dispatch(Action::ShowQuickActions);
            }
            "/find-file" | "/ff" => {
                self.dispatch(Action::ShowFuzzyFinder);
            }
            "/grep" | "/content-search" => {
                let dir = self.active_tree_ref().root.clone();
                self.search = Some(SearchState::new_content_focused(dir));
            }
            "/diff" | "/compare-files" => {
                self.dispatch(Action::DiffFiles);
            }
            "/ssh" => {
                if args.is_empty() {
                    self.feedback
                        .error("Usage: /ssh user@host:/path".to_string());
                } else {
                    self.dispatch(Action::SshBrowse(args.to_string()));
                }
            }
            "/duplicates" | "/dupes" => {
                self.dispatch(Action::FindDuplicates);
            }
            "/treemap" | "/usage" => {
                self.dispatch(Action::ShowTreemap);
            }
            "/rename-batch" | "/bulk-rename" => {
                self.dispatch(Action::BatchRename);
            }
            "/undo" => {
                self.dispatch(Action::Undo);
            }
            "/extract" => {
                self.dispatch(Action::ExtractArchive);
            }
            "/compress" | "/zip" => {
                self.dispatch(Action::CompressSelection);
            }
            "/plugin" => {
                if args.is_empty() {
                    // List plugin commands
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
                } else {
                    // Run a plugin command
                    let dir = self.active_tree_ref().root.to_string_lossy().to_string();
                    if let Some(ref engine) = self.plugin_engine {
                        match engine.execute_command(args, &dir) {
                            Ok(farx_plugin::PluginResult::Message(msg)) => {
                                self.feedback.info(msg);
                            }
                            Ok(farx_plugin::PluginResult::Shell(cmd)) => {
                                self.command_line.input = cmd;
                                self.smart_execute_command();
                            }
                            Ok(farx_plugin::PluginResult::None) => {}
                            Err(e) => {
                                self.feedback.error(format!("Plugin: {}", e));
                            }
                        }
                    }
                }
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
            "/symlink" | "/ln" => {
                self.dispatch(Action::CreateSymlinkDialog);
            }
            "/select" => {
                if args.is_empty() {
                    self.dispatch(Action::SelectByMaskDialog);
                } else {
                    self.apply_mask_selection(args, true);
                }
            }
            "/deselect" => {
                if args.is_empty() {
                    self.dispatch(Action::DeselectByMaskDialog);
                } else {
                    self.apply_mask_selection(args, false);
                }
            }
            "/invert" => {
                self.dispatch(Action::InvertSelection);
            }
            "/compare" | "/cmp" => {
                self.dispatch(Action::CompareDirectories);
            }
            "/swap" => {
                self.dispatch(Action::SwapPanels);
            }
            "/open" => {
                self.dispatch(Action::OpenSystemApp);
            }
            "/terminal" | "/term" => {
                self.dispatch(Action::OpenTerminalHere);
            }
            "/recent" | "/history" => {
                self.dispatch(Action::ShowRecentDirectories);
            }
            "/touch" => {
                self.dispatch(Action::TouchFile);
            }
            "/stats" | "/wc" => {
                self.dispatch(Action::ShowFileStats);
            }
            "/goto" | "/go" | "/g" => {
                if args.is_empty() {
                    self.dispatch(Action::GotoDirectoryDialog);
                } else {
                    // Direct navigation
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
            _ => {
                // Try plugin commands: /cmd_name → plugin "cmd_name"
                let plugin_cmd = cmd.trim_start_matches('/');
                if let Some(ref engine) = self.plugin_engine {
                    if engine.has_command(plugin_cmd) {
                        let dir = self.active_tree_ref().root.to_string_lossy().to_string();
                        match engine.execute_command(plugin_cmd, &dir) {
                            Ok(farx_plugin::PluginResult::Message(msg)) => {
                                self.feedback.info(msg);
                            }
                            Ok(farx_plugin::PluginResult::Shell(shell_cmd)) => {
                                self.command_line.input = shell_cmd;
                                self.smart_execute_command();
                            }
                            Ok(farx_plugin::PluginResult::None) => {}
                            Err(e) => {
                                self.feedback.error(format!("Plugin: {}", e));
                            }
                        }
                        return true;
                    }
                }
                return false;
            }
        }
        true
    }

    /// Parse a `cd` command, returning the argument (possibly empty for bare `cd`).
    /// Returns `None` if the input is not a `cd` command.
    fn parse_cd_command(input: &str) -> Option<String> {
        let trimmed = input.trim();
        if trimmed == "cd" {
            return Some(String::new());
        }
        if let Some(rest) = trimmed.strip_prefix("cd ") {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = trimmed.strip_prefix("cd\t") {
            return Some(rest.trim().to_string());
        }
        None
    }

    /// Update the slash command suggestion popup based on current input.
    fn update_slash_suggestions(&mut self) {
        let input = &self.command_line.input;
        // Only show suggestions when input is just the command prefix (no args yet)
        if input.starts_with('/') && !input.contains(' ') {
            let state = SlashSuggestionsState::new(input);
            if state.matches.is_empty() {
                self.slash_suggestions = None;
            } else {
                self.slash_suggestions = Some(state);
            }
        } else {
            self.slash_suggestions = None;
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
        // Handle non-filesystem operations
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
            PendingOperation::CopySameDir { source } => {
                if let Some(name) = input_value {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let dest_dir = self.active_tree_ref().root.clone();
                    let dest = dest_dir.join(name);
                    std::fs::copy(&source, &dest)
                        .map(|_| ())
                        .map_err(anyhow::Error::from)
                } else {
                    return;
                }
            }
            PendingOperation::CreateSymlink { target } => {
                if let Some(name) = input_value {
                    let name = name.trim();
                    if name.is_empty() {
                        return;
                    }
                    let link_dir = self.active_tree_ref().root.clone();
                    let link_path = link_dir.join(name);
                    farx_fs::create_symlink(&target, &link_path)
                } else {
                    return;
                }
            }
            // Already handled above; included for exhaustiveness
            PendingOperation::SelectByMask
            | PendingOperation::DeselectByMask
            | PendingOperation::GotoDirectory => return,
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
                if sources.len() >= 2 {
                    // Use progress dialog for multi-file copy
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
                    // Use progress dialog for multi-file move
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
                    } else if farx_fs::is_archive(&path) {
                        // Archive: browse contents
                        self.dispatch(Action::ViewArchive);
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
            Action::SelectPageUp => {
                let tree = self.active_tree();
                let cursor = tree.cursor;
                let target = cursor.saturating_sub(20);
                for i in (target..cursor).rev() {
                    if i < tree.visible_nodes.len() && tree.visible_nodes[i].entry.name != ".." {
                        tree.selected.insert(i);
                    }
                }
                tree.move_cursor_to(target);
                return;
            }
            Action::SelectPageDown => {
                let tree = self.active_tree();
                let cursor = tree.cursor;
                let max = tree.visible_nodes.len().saturating_sub(1);
                let target = (cursor + 20).min(max);
                for i in cursor..=target {
                    if i < tree.visible_nodes.len() && tree.visible_nodes[i].entry.name != ".." {
                        tree.selected.insert(i);
                    }
                }
                tree.move_cursor_to(target);
                return;
            }
            Action::SelectHome => {
                let tree = self.active_tree();
                let cursor = tree.cursor;
                for i in 0..cursor {
                    if i < tree.visible_nodes.len() && tree.visible_nodes[i].entry.name != ".." {
                        tree.selected.insert(i);
                    }
                }
                tree.move_cursor_to(0);
                return;
            }
            Action::SelectEnd => {
                let tree = self.active_tree();
                let cursor = tree.cursor;
                let max = tree.visible_nodes.len().saturating_sub(1);
                for i in cursor..=max {
                    if i < tree.visible_nodes.len() && tree.visible_nodes[i].entry.name != ".." {
                        tree.selected.insert(i);
                    }
                }
                tree.move_cursor_to(max);
                return;
            }
            Action::SelectAll => {
                let tree = self.active_tree();
                for i in 0..tree.visible_nodes.len() {
                    if tree.visible_nodes[i].entry.name != ".." {
                        tree.selected.insert(i);
                    }
                }
                return;
            }
            Action::DeselectAll => {
                self.active_tree().selected.clear();
                return;
            }
            Action::InvertSelection => {
                let tree = self.active_tree();
                for i in 0..tree.visible_nodes.len() {
                    if tree.visible_nodes[i].entry.name != ".." {
                        if tree.selected.contains(&i) {
                            tree.selected.remove(&i);
                        } else {
                            tree.selected.insert(i);
                        }
                    }
                }
                return;
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
                self.cycle_focus();
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

    /// Render the entire application UI into the given frame.
    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Full-screen modals first
        if let Some(ref editor) = self.editor {
            render_editor(frame, editor, &self.theme);
            return;
        }
        if let Some(ref mut viewer) = self.viewer {
            render_viewer(frame, viewer, &self.theme);
            return;
        }
        if let Some(ref help) = self.help {
            render_help(frame, help, &self.theme);
            return;
        }
        if let Some(ref diff) = self.diff_view {
            render_diff_view(frame, diff, &self.theme);
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

        // Compute panel rects from the layout tree
        let panel_rects = self.layout.compute_rects(main_chunks[0]);
        self.cached_panel_rects = panel_rects.clone();

        // Cache fn-bar rect for mouse hit-testing
        if self.config.ui.show_fn_bar {
            self.cached_fn_bar_rect = Some(main_chunks[2]);
        } else {
            self.cached_fn_bar_rect = None;
        }

        // Render each leaf panel
        for (leaf, rect) in &panel_rects {
            match leaf {
                farx_core::PanelLeaf::FilePanel(side) => {
                    let (tabs, tree, panel_state) = match side {
                        PanelSide::Left => (
                            self.left_tree.tab_labels(),
                            &mut self.left_tree as &mut TabGroup,
                            &self.left_panel,
                        ),
                        PanelSide::Right => (
                            self.right_tree.tab_labels(),
                            &mut self.right_tree as &mut TabGroup,
                            &self.right_panel,
                        ),
                    };
                    let is_active = self.focused_terminal.is_none() && self.active_panel == *side;
                    let filter_editing = is_active && self.filter_active;

                    // Render tab bar (consumes 0 or 1 row)
                    let tab_height = render_tab_bar(frame, *rect, &tabs, is_active, &self.theme);
                    let panel_rect = ratatui::layout::Rect {
                        y: rect.y + tab_height,
                        height: rect.height.saturating_sub(tab_height),
                        ..*rect
                    };

                    // Scroll adjustments
                    let panel_height = panel_rect.height.saturating_sub(3) as usize;
                    tree.scroll_to_cursor(panel_height);

                    if self.show_info_panel && *side != self.active_panel {
                        let current_file = self.active_tree_ref().current_node().map(|n| &n.entry);
                        let data = InfoPanelData::from_panel(panel_state, current_file);
                        render_info_panel(frame, panel_rect, &data, &self.theme);
                    } else {
                        render_tree_panel_with_filter(
                            frame,
                            panel_rect,
                            tree,
                            is_active,
                            &self.theme,
                            filter_editing,
                        );
                    }
                }
                farx_core::PanelLeaf::Terminal(tid) => {
                    if let Some(term) = self.terminals.get_mut(*tid) {
                        // Resize terminal to match panel inner area
                        let inner_h = rect.height.saturating_sub(2);
                        let inner_w = rect.width.saturating_sub(2);
                        if inner_h > 0 && inner_w > 0 {
                            term.resize(inner_h, inner_w);
                        }
                        let is_focused = self.focused_terminal == Some(*tid);
                        render_terminal(frame, *rect, term, is_focused);
                    }
                }
            }
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

        // Slash command suggestions popup (floats above command line)
        if let Some(ref ss) = self.slash_suggestions {
            render_slash_suggestions(frame, ss, main_chunks[1]);
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

        // Fuzzy finder
        if let Some(ref ff) = self.fuzzy_finder {
            render_fuzzy_finder(frame, ff, &self.theme);
        }

        // Quick actions palette
        if let Some(ref qa) = self.quick_actions {
            render_quick_actions(frame, qa, &self.theme);
        }

        // AI tools panel
        if let Some(ref ai_panel) = self.ai_panel {
            render_ai_panel(frame, ai_panel, &self.theme);
        }

        // Batch rename dialog
        if let Some(ref br) = self.batch_rename {
            render_batch_rename(frame, br, &self.theme);
        }

        // Chmod dialog
        if let Some(ref chmod) = self.chmod_dialog {
            render_chmod_dialog(frame, chmod, &self.theme);
        }

        // Dialog only for text input (MkDir, Rename, CreateFile)
        if let Some(ref dialog) = self.dialog {
            render_dialog(frame, dialog, &self.theme);
        }

        // Progress dialog (renders on top of everything)
        if let Some(ref progress) = self.progress {
            render_progress(frame, progress, &self.theme);
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

/// Simple glob pattern matcher supporting `*` (any chars) and `?` (single char).
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_match_impl(&p, &t)
}

fn glob_match_impl(pattern: &[char], text: &[char]) -> bool {
    let (mut pi, mut ti) = (0, 0);
    let (mut star_pi, mut star_ti) = (usize::MAX, 0);

    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == '*' {
            star_pi = pi;
            star_ti = ti;
            pi += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }
    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }
    pi == pattern.len()
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
            | "heic" | "heif" | "raw" | "cr2" | "nef"
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
