//! App constructor and the panel refresh helpers it depends on.

use farx_core::{AppConfig, KeyMap, PanelSide, PanelState, TabGroup, TreeState};

use crate::components::bookmarks::load_bookmarks;
use crate::components::command_line::CommandLineState;
use crate::components::feedback::FeedbackState;
use crate::theme::Theme;

use super::App;

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

        Self::refresh_panel(&mut left, show_hidden);
        Self::refresh_panel(&mut right, show_hidden);

        let ai_agent = farx_ai::AiAgent::new(
            &config.ai.provider,
            config.ai.base_url.clone(),
            config.ai.model.clone(),
            config.ai.fallback_models.clone(),
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
            focused_terminal: None,
            next_terminal_id: 0,
            grid: farx_core::GridLayout::new(),
            terminal_event_tx: None,
            output_pending: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            fs_watcher: None,
            fs_change_rx: None,
            fs_change_tick: 0,
            cached_panel_rects: Vec::new(),
            last_click: None,
        };

        app.setup_fs_watcher();
        Ok(app)
    }

    /// Re-read the directory listing for a panel and sort the entries.
    pub(super) fn refresh_panel(panel: &mut PanelState, show_hidden: bool) {
        if let Ok(entries) = farx_fs::read_directory(&panel.current_dir, show_hidden) {
            panel.entries = entries;
            panel.sort_entries();
        }
    }

    /// Refresh both panels.
    pub(super) fn refresh_both_panels(&mut self) {
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
}
