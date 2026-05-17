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

use farx_core::Action;

pub use self::state::App;

impl App {
    /// Execute an action, updating application state accordingly.
    pub fn dispatch(&mut self, action: Action) {
        if self.dispatch_tree_nav(&action) || self.dispatch_selection(&action) {
            return;
        }
        let _ = self.dispatch_control(&action)
            || self.dispatch_file_dialogs(&action)
            || self.dispatch_cmdline(&action)
            || self.dispatch_nav(&action)
            || self.dispatch_clipboard(&action)
            || self.dispatch_term_palette(&action)
            || self.dispatch_bulk_ops(&action)
            || self.dispatch_archives(&action)
            || self.dispatch_analysis(&action);
    }
}
