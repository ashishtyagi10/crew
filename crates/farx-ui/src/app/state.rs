//! Central application state — every field of the `App` struct lives
//! here. Method `impl App { ... }` blocks are spread across this module's
//! sibling files.

use farx_core::{GridLayout, KeyMap, PanelSide, PanelState, TabGroup};

use crate::components::ai_bar::AiBarState;
use crate::components::ai_panel::AiPanelState;
use crate::components::batch_rename::BatchRenameState;
use crate::components::bookmarks::{Bookmark, BookmarkState};
use crate::components::chmod_dialog::ChmodDialogState;
use crate::components::command_line::CommandLineState;
use crate::components::dialog::DialogState;
use crate::components::diff_view::DiffViewState;
use crate::components::editor::EditorState;
use crate::components::feedback::FeedbackState;
use crate::components::fuzzy_finder::FuzzyFinderState;
use crate::components::help::HelpState;
use crate::components::menu::MenuState;
use crate::components::progress::ProgressState;
use crate::components::quick_actions::QuickActionsState;
use crate::components::search::SearchState;
use crate::components::slash_suggestions::SlashSuggestionsState;
use crate::components::update_modal::UpdateState;
use crate::components::viewer::ViewerState;
use crate::theme::Theme;

use farx_core::AppConfig;

use super::pending::{PendingOperation, UndoEntry};

/// Main application state that owns panels, config, and the render loop.
pub struct App {
    pub running: bool,
    pub active_panel: PanelSide,
    pub left_panel: PanelState,
    pub right_panel: PanelState,
    pub command_line: CommandLineState,
    pub panels_visible: bool,
    pub config: AppConfig,
    pub keymap: KeyMap,
    pub theme: Theme,
    pub dialog: Option<DialogState>,
    pub(super) pending_op: Option<PendingOperation>,
    pub viewer: Option<ViewerState>,
    pub help: Option<HelpState>,
    pub ai_bar: Option<AiBarState>,
    pub(super) ai_agent: farx_ai::AiAgent,
    pub(super) ai_pending_response: Option<tokio::sync::oneshot::Receiver<String>>,
    pub editor: Option<EditorState>,
    pub menu: Option<MenuState>,
    pub search: Option<SearchState>,
    pub show_info_panel: bool,
    pub command_output: Option<String>,
    pub feedback: FeedbackState,
    pub(super) tick_count: u64,
    pub(super) suggestion_rx: Option<tokio::sync::oneshot::Receiver<Option<String>>>,
    pub(super) suggestion_request_input: String,
    pub left_tree: TabGroup,
    pub right_tree: TabGroup,
    pub update_available: Option<String>,
    pub bookmarks_panel: Option<BookmarkState>,
    pub bookmarks: Vec<Bookmark>,
    pub filter_active: bool,
    pub filter_pattern: String,
    pub plugin_engine: Option<farx_plugin::PluginEngine>,
    pub(super) undo_stack: Vec<UndoEntry>,
    pub batch_rename: Option<BatchRenameState>,
    pub chmod_dialog: Option<ChmodDialogState>,
    pub progress: Option<ProgressState>,
    pub diff_view: Option<DiffViewState>,
    pub fuzzy_finder: Option<FuzzyFinderState>,
    pub quick_actions: Option<QuickActionsState>,
    pub ai_panel: Option<AiPanelState>,
    pub slash_suggestions: Option<SlashSuggestionsState>,
    pub update_state: Option<UpdateState>,
    pub pending_install: bool,
    pub terminals: Vec<crate::components::embedded_terminal::TerminalSession>,
    pub focused_terminal: Option<usize>,
    /// Monotonic counter for stable terminal ids.
    pub(super) next_terminal_id: usize,
    /// LRU-ordered grid layout tracking which terminals are full vs minimized.
    pub(super) grid: GridLayout,
    /// Sender used by PTY reader threads to wake the render loop on output.
    pub(super) terminal_event_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::event::Event>>,
    /// Coalescing flag: set when a terminal-output wake-up is already queued,
    /// cleared when the loop drains terminals, to avoid redraw storms.
    pub(super) output_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(super) fs_watcher: Option<notify::RecommendedWatcher>,
    pub(super) fs_change_rx: Option<std::sync::mpsc::Receiver<()>>,
    pub(super) fs_change_tick: u64,
    pub(super) cached_panel_rects: Vec<(farx_core::PanelLeaf, ratatui::layout::Rect)>,
    pub(super) last_click: Option<(u16, u16, u64)>,
}
