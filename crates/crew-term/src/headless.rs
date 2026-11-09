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

    /// Pictures the fed program dropped (see [`crate::ImageWipe`]).
    pub fn take_image_wipes(&mut self) -> Vec<crate::ImageWipe> {
        self.core.take_image_wipes()
    }

    /// Scrollback above the screen.
    pub fn history_lines(&self) -> usize {
        self.core.history_lines()
    }

    /// The absolute line of the screen's top row, for placing those pictures
    /// (see [`crate::PlacedImage::line`]).
    pub fn screen_line(&self) -> u64 {
        self.core.screen_line()
    }

    /// Keep `lines` of scrollback (alacritty's default is 10 000).
    pub fn set_scrollback(&mut self, lines: usize) {
        self.core.set_scrollback(lines);
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

    /// The newest line at or above depth `from` that `hit` accepts (see
    /// `scrollfind`).
    pub fn find_back(&self, from: usize, hit: &mut dyn FnMut(&str) -> bool) -> Option<usize> {
        self.core.find_back(from, hit)
    }

    /// The DEC private modes the program has on (wheel, keys, focus).
    pub fn input_modes(&self) -> crate::modes::InputModes {
        self.core.input_modes()
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

    fn last_line(&self) -> Option<String> {
        self.core.last_line()
    }

    fn wrapped_rows(&self) -> Vec<bool> {
        self.core.wrapped_rows()
    }

    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
    }
}
