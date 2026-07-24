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

    pub fn take_cwd(&mut self) -> Option<std::path::PathBuf> {
        self.core.take_cwd()
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
