mod accessors;
mod ai_glue;
mod chrome;
mod commands;
mod confirm;
mod dialogs;
mod fs_watcher;
mod globs;
mod helpers;
mod keys;
mod mouse;
mod pending;
mod selection_ops;
mod shell_commands;
mod slash;
mod terminals;
mod text_detect;
mod tick;
mod tools;
mod update_flow;

use std::path::PathBuf;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use farx_core::{Action, AppConfig, KeyMap, PanelSide, PanelState, TabGroup, TreeState};

use farx_core::SortField;

use crate::components::ai_bar::{render_ai_bar, AiBarState};
use crate::components::ai_panel::{render_ai_panel, AiPanelState};
use crate::components::batch_rename::{render_batch_rename, BatchRenameState};
use crate::components::bookmarks::{
    load_bookmarks, render_bookmarks, save_bookmarks, Bookmark, BookmarkState,
};
use crate::components::chmod_dialog::{render_chmod_dialog, ChmodDialogState};
use crate::components::command_line::CommandLineState;
use crate::components::dialog::{render_dialog, DialogState};
use crate::components::diff_view::{render_diff_view, DiffViewState};
use crate::components::editor::{render_editor, EditorState};
use crate::components::embedded_terminal::render_terminal;
use crate::components::feedback::{render_feedback, ConfirmAction, FeedbackState};
use crate::components::fuzzy_finder::{render_fuzzy_finder, FuzzyFinderState};
use crate::components::help::{render_help, HelpState};
use crate::components::info_panel::{render_info_panel, InfoPanelData};
use crate::components::menu::{render_menu, MenuState};
use crate::components::progress::{render_progress, ProgressState};
use crate::components::quick_actions::{render_quick_actions, QuickActionsState};
use crate::components::search::{render_search, SearchState};
use crate::components::slash_suggestions::{render_slash_suggestions, SlashSuggestionsState};
use crate::components::tree_panel::{render_tab_bar, render_tree_panel_with_filter};
use crate::components::update_modal::render_update_modal;
use crate::components::viewer::{render_viewer, ViewerState};
use crate::components::{command_line, fn_bar};
use crate::theme::Theme;

use self::helpers::format_size_human;
use self::pending::{PendingOperation, UndoEntry};
use self::text_detect::is_text_file;

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
    /// In-TUI auto-update flow state (`/update` command).
    pub update_state: Option<crate::components::update_modal::UpdateState>,
    /// Set to true when the main loop should leave the alternate screen and
    /// run the blocking installer. Cleared by the main loop after handling.
    pub pending_install: bool,
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
            keymap: {
                let mut km = KeyMap::far_defaults();
                if !config.keybindings.is_empty() {
                    km.apply_overrides(&config.keybindings);
                }
                km
            },
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
            update_state: None,
            pending_install: false,
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

    /// Handle mouse events (scroll, click, double-click).

    /// Handle a click on the function key bar.

    /// Smart command execution: detects whether the input is a shell command or
    /// natural language, and routes accordingly.
    ///
    /// Heuristic: if the input starts with a known command/path prefix, or contains

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

    /// Render the entire application UI into the given frame.
    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Full-screen modals first
        if let Some(ref mut editor) = self.editor {
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

        // Layout: panels | status bar (1 row) | command box (3 rows) | fn bar (1 row)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Panels
                Constraint::Length(1), // Status bar
                Constraint::Length(3), // Command line box
                Constraint::Length(1), // Function key bar
            ])
            .split(size);

        // Render status bar
        self.render_status_bar(frame, main_chunks[1]);

        // Compute panel rects from the layout tree
        let panel_rects = self.layout.compute_rects(main_chunks[0]);
        self.cached_panel_rects = panel_rects.clone();

        // Cache fn-bar rect for mouse hit-testing
        if self.config.ui.show_fn_bar {
            self.cached_fn_bar_rect = Some(main_chunks[3]);
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
            render_feedback(frame, main_chunks[2], &self.feedback);
        } else {
            let active_dir = self.active_tree_ref().root.clone();
            command_line::render_command_line(
                frame,
                main_chunks[2],
                &self.command_line,
                &active_dir,
                &self.theme,
            );
        }

        // Slash command suggestions popup (floats above command line)
        if let Some(ref ss) = self.slash_suggestions {
            render_slash_suggestions(frame, ss, main_chunks[2]);
        }

        // Render function key bar
        if self.config.ui.show_fn_bar {
            fn_bar::render_fn_bar(frame, main_chunks[3], &self.theme);
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

        // Update modal renders last so it sits above every other overlay.
        if let Some(ref state) = self.update_state {
            render_update_modal(frame, state, &self.theme);
        }
    }
}
