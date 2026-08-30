//! `HeadlessTerm`: the windowless `TermModel` used by tests and tools.
//! Split from `model.rs` (child module — parent-private access preserved).
use super::*;

pub struct HeadlessTerm {
    core: TermCore,
}

impl HeadlessTerm {
    pub fn new(size: GridSize) -> Self {
        Self {
            core: TermCore::new(size),
        }
    }

    pub fn scroll(&mut self, delta: i32) {
        self.core.scroll(delta);
    }

    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
    }

    pub fn title(&self) -> String {
        self.core.title()
    }

    /// Pictures the fed program asked to show (see [`crate::graphics`]).
    pub fn take_images(&mut self) -> Vec<crate::model::PlacedImage> {
        self.core.take_images()
    }

    /// Scrollback above the screen, for placing those pictures.
    pub fn history_lines(&self) -> usize {
        self.core.history_lines()
    }

    /// Publish the frame's cell size, as the app does on resize.
    pub fn set_cell_px(&mut self, w: u32, h: u32) {
        self.core.set_cell_px(w, h);
    }

    pub fn take_shell(&mut self) -> Vec<crate::osc::ShellMark> {
        self.core.take_shell()
    }

    pub fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.core.take_cwd()
    }

    /// Whether the fed program enabled DECSET 2031 (scheme-change reports).
    pub fn scheme_notify_enabled(&self) -> bool {
        self.core.scheme_notify_enabled()
    }

    /// A notification the program in this pane asked for (OSC 9 / OSC 777),
    /// once.
    pub fn take_notify(&mut self) -> Option<(String, String)> {
        self.core.take_notify()
    }

    /// What the program says about its own progress (OSC 9;4).
    pub fn progress(&self) -> Option<crate::osc::Progress> {
        self.core.progress()
    }

    /// The OSC 8 hyperlink target under viewport cell `(col, row)`.
    pub fn link_at(&self, col: u16, row: u16) -> Option<String> {
        self.core.link_at(col, row)
    }

    pub fn take_bell(&self) -> bool {
        self.core.take_bell()
    }

    pub fn take_clipboard(&self) -> Option<String> {
        self.core.take_clipboard()
    }

    /// Take pending query replies (OSC color / DSR reports) owed to the child.
    pub fn take_replies(&self) -> Option<String> {
        self.core.take_replies()
    }
}

impl HeadlessTerm {
    pub fn sel_start(&mut self, col: u16, row: u16, block: bool) {
        self.core.sel_start(col, row, block);
    }

    pub fn sel_update(&mut self, col: u16, row: u16) {
        self.core.sel_update(col, row);
    }

    pub fn sel_word(&mut self, col: u16, row: u16) {
        self.core.sel_word(col, row);
    }

    pub fn sel_line(&mut self, col: u16, row: u16) {
        self.core.sel_line(col, row);
    }

    pub fn sel_clear(&mut self) {
        self.core.sel_clear();
    }

    pub fn sel_text(&self) -> Option<String> {
        self.core.sel_text()
    }
}

impl TermModel for HeadlessTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }

    fn cells(&self, focused: bool) -> Vec<RenderCell> {
        self.core.cells(focused)
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
    }
}
