//! A document in a window of its own.
//!
//! Crew is one window holding a grid of panes, and that is the right shape for
//! the work — a shell beside an agent beside a diff. It is the wrong shape for
//! the one thing you read for twenty minutes at a time: a document wants a
//! window you can put on the other screen, size to a comfortable measure, and
//! leave open while the grid goes on being a grid.
//!
//! So a document window is exactly that and nothing more: no nav, no input
//! bar, no tiles — one file, framed, filling its own window. It is a second
//! *surface*, not a second app; the process, the broker, the theme, the config
//! and the font database are all still one.
//!
//! **What this deliberately is not:** a second canvas. Pillar 1 of
//! `docs/superpowers/goals/2026-08-30-markdown-editor-in-its-own-window.md`
//! makes windows plural all the way down — panes, focus, zoom, the lot — and
//! that is a refactor of two hundred call sites. A document window needs none
//! of it, because it holds no panes, and it is the half the reader actually
//! asked for.
use std::path::PathBuf;
use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::viewpane::ViewPane;
use crew_render::Renderer;
use crew_term::GridSize;

pub(crate) mod draw;
mod event;
pub(crate) mod keys;
pub(crate) mod linkedit;

/// The margin, in cells, between the window's edge and the document's frame —
/// the same one-cell ring every pane card sits in.
const MARGIN: f32 = 12.0;

/// One open document window.
pub(crate) struct DocWindow {
    pub window: Arc<Window>,
    pub renderer: Renderer,
    pub view: ViewPane,
    /// This window's own modifier state — it is a separate surface, so the
    /// grid's never reaches it.
    pub mods: winit::keyboard::ModifiersState,
    /// Whether an Esc on unsaved changes has already been refused once.
    pub warned: bool,
    /// Last pointer position, in this window's physical pixels — winit
    /// reports the position and the button press as separate events.
    pub pointer: (f32, f32),
    /// The URL field, while it is open (Cmd+K).
    pub link: Option<linkedit::UrlEdit>,
    /// One line the window has to say for itself, shown on the frame until the next key. A
    /// document window is its own surface: the main window's status bar is somewhere the person
    /// reading this is not looking.
    pub hint: Option<&'static str>,
    /// The grid the document was last laid out at. Recomputed on every resize,
    /// because a document wraps to its window and nothing else.
    pub grid: GridSize,
}

impl DocWindow {
    /// Open `path` in a new window. `None` when the window or its surface
    /// could not be created — a failed pop-out must leave the app alone, not
    /// take it down.
    pub(crate) fn open(
        event_loop: &ActiveEventLoop,
        path: PathBuf,
        font_size: f32,
    ) -> Option<Self> {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let attrs = Window::default_attributes()
            .with_title(format!("{title} \u{2014} crew"))
            .with_resizable(true)
            // A measure, not a canvas: tall and narrow-ish is what a document
            // is read at, and it is what makes the window read as a document
            // the moment it appears.
            .with_inner_size(winit::dpi::LogicalSize::new(820.0, 900.0));
        let window = Arc::new(event_loop.create_window(attrs).ok()?);
        let renderer = Renderer::new(Arc::clone(&window), font_size).ok()?;
        let mut me = Self {
            window,
            renderer,
            view: ViewPane::open(path),
            mods: Default::default(),
            warned: false,
            pointer: (0.0, 0.0),
            link: None,
            hint: None,
            grid: GridSize { cols: 80, rows: 40 },
        };
        me.refit();
        Some(me)
    }

    /// The document row and column under a pointer position, or `None` when
    /// it is on the frame rather than in the text.
    pub(crate) fn cell_at(&self, (x, y): (f32, f32)) -> Option<(usize, u16)> {
        let (cw, ch) = self.renderer.cell_size();
        let scale = self.window.scale_factor() as f32;
        // The document starts one cell inside the frame, which itself starts
        // `MARGIN` from the window's edge (see `draw`).
        let (x0, y0) = (MARGIN * scale + cw, MARGIN * scale + ch);
        if x < x0 || y < y0 || cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        let col = ((x - x0) / cw) as u16;
        let row = ((y - y0) / ch) as usize;
        (col < self.grid.cols && row < usize::from(self.grid.rows)).then_some((row, col))
    }

    pub(crate) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Recompute the document's grid from the window's size. The frame takes
    /// a one-cell ring, exactly as a pane card does.
    pub(crate) fn refit(&mut self) {
        let (w, h) = self.renderer.surface_size();
        let (cw, ch) = self.renderer.cell_size();
        let scale = self.window.scale_factor() as f32;
        let (iw, ih) = (
            w as f32 - MARGIN * 2.0 * scale,
            h as f32 - MARGIN * 2.0 * scale,
        );
        let (cols, rows) = crate::layout::card_inner_cells(iw, ih, cw, ch);
        self.grid = GridSize { cols, rows };
        self.view.clamp_scroll(cols, rows);
        // The document just re-wrapped: the caret's row and column belonged
        // to the old width, and only its byte survives the change.
        self.view.relayout_caret(cols, rows);
    }

    /// Drain the worker that is loading the file. Returns whether anything
    /// changed — the document window's whole reason to want a frame while
    /// nobody is typing.
    pub(crate) fn poll(&mut self) -> bool {
        if !self.view.poll() {
            return false;
        }
        // The file has landed: if it is a document rather than a listing of
        // bytes, it opens with a cursor already in it. That IS the difference
        // between a viewer and an editor, and it is why a document window
        // shows a caret and a viewer pane does not.
        if self.editable() {
            self.view.start_editing(self.grid.cols);
        }
        true
    }

    /// Whether this window holds something the caret belongs in. Markdown
    /// only, for now: the rung whose render carries the provenance a cursor
    /// is made of (see `crate::md::source`).
    pub(crate) fn editable(&self) -> bool {
        matches!(
            &self.view.state,
            crate::viewpane::LoadState::Ready {
                format: crate::viewpane::detect::Format::Markdown,
                ..
            }
        ) && !self.view.raw
    }
}

#[cfg(test)]
#[path = "docwin_tests.rs"]
mod docwin_tests;

#[cfg(test)]
#[path = "savediff_tests.rs"]
mod savediff_tests;
