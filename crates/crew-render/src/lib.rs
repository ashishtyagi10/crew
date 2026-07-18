//! crew-render: winit window + wgpu surface + glyphon text.
mod cellgrid;
mod celltext;
pub mod color;
mod crt;
mod fontlist;
mod frame;
mod gpu;
mod paperbg;
mod quads;
mod renderer;
mod roundborder;
mod scene;
mod scenecache;
mod scenetarget;
mod textprep;
pub use cellgrid::CellGrid;
pub use cellgrid::CellView;
pub use crt::CrtPass;
pub use paperbg::PaperBgPass;
pub use renderer::Renderer;
pub use scene::PaneScene;

/// Sorted, de-duplicated names of every installed monospace font family —
/// flagged/name-matched candidates verified to render fixed-pitch Latin (see
/// [`fontlist`]). GPU-free (builds its own font database), so diagnostics
/// like `crew --list-fonts` can call it without a window.
pub fn list_monospace_families() -> Vec<String> {
    fontlist::monospace_families(&mut glyphon::FontSystem::new())
}
