//! `w`: the open thread in a window of its own (goal 2026-10-20-mail-reader,
//! item 16) — one you can put on the other screen and size to a comfortable
//! measure, while the grid goes on being a grid.
//!
//! It is the inbox pane drawn a second time, at the window's size: the same
//! cells, the same pictures, the same keys, all going to the one pane. So
//! `e`, `d`, `r` and the rest act in the window exactly as in the tile, and
//! the two can never disagree about what is open. The document window
//! (`docwin`) is built around the file viewer and its editor; this borrows
//! its frame and its margin, not its keys.
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crew_render::{PaneScene, Renderer};
use crew_term::GridSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use super::InboxPane;
use crate::layout::Rect;

/// The ring between the window's edge and the card, as a document window has.
const MARGIN: f32 = 12.0;

pub(crate) struct MailWindow {
    pub window: Arc<Window>,
    pub renderer: Renderer,
    pub mods: winit::keyboard::ModifiersState,
    pub grid: GridSize,
    /// Last pointer position, in physical pixels.
    pub pointer: (f32, f32),
    /// A fingerprint of what was last drawn ([`look`]).
    pub seen: u64,
}

impl MailWindow {
    pub(crate) fn open(event_loop: &ActiveEventLoop, font_size: f32, title: &str) -> Option<Self> {
        let attrs = Window::default_attributes()
            .with_title(format!("{title} \u{2014} crew"))
            .with_resizable(true)
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 1000.0));
        let window = Arc::new(event_loop.create_window(attrs).ok()?);
        let renderer = Renderer::new(Arc::clone(&window), font_size).ok()?;
        let mut me = Self {
            window,
            renderer,
            mods: Default::default(),
            grid: GridSize { cols: 80, rows: 40 },
            pointer: (0.0, 0.0),
            seen: 0,
        };
        me.refit();
        Some(me)
    }

    pub(crate) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// The card's inner grid from the window's size.
    pub(crate) fn refit(&mut self) {
        let (w, h) = self.renderer.surface_size();
        let (cw, ch) = self.renderer.cell_size();
        let m = MARGIN * 2.0 * self.window.scale_factor() as f32;
        let (cols, rows) = crate::layout::card_inner_cells(w as f32 - m, h as f32 - m, cw, ch);
        self.grid = GridSize { cols, rows };
    }

    /// The pane's row and column under the pointer, inside the card.
    pub(crate) fn cell_at(&self) -> Option<(u16, u16)> {
        let (cw, ch) = self.renderer.cell_size();
        let s = self.window.scale_factor() as f32;
        let (x, y) = (
            self.pointer.0 - MARGIN * s - cw,
            self.pointer.1 - MARGIN * s - ch,
        );
        if x < 0.0 || y < 0.0 || cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        let (col, row) = ((x / cw) as u16, (y / ch) as u16);
        (col < self.grid.cols && row < self.grid.rows).then_some((row, col))
    }

    pub(crate) fn draw(&mut self, pane: &InboxPane) {
        let (w, h) = self.renderer.surface_size();
        if w == 0 || h == 0 {
            return;
        }
        let (cw, ch) = self.renderer.cell_size();
        let m = MARGIN * self.window.scale_factor() as f32;
        let rect = Rect {
            x: m,
            y: m,
            w: w as f32 - m * 2.0,
            h: h as f32 - m * 2.0,
        };
        let mut scenes: Vec<PaneScene> = Vec::new();
        crate::panelcard::push_card_art(
            &mut scenes,
            rect,
            cw,
            ch,
            &pane.legend(),
            crew_theme::theme().legend_off,
            |cols, rows| pane.art(cols, rows, ch / cw),
        );
        self.renderer.set_theme_fade(None);
        self.renderer.set_solid_chrome(Vec::new());
        self.renderer.frame(&scenes);
    }
}

/// What the pane would draw at `grid`, as a number: when it moves, the
/// window draws again. Cells and the pictures that have landed are all a
/// frame can show, so they are all this reads.
pub(crate) fn look(pane: &InboxPane, grid: GridSize) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for c in pane.cells(grid.cols, grid.rows) {
        (c.row, c.col, c.c, c.fg, c.bg, c.bold).hash(&mut h);
    }
    pane.pics.rev.hash(&mut h);
    h.finish()
}
