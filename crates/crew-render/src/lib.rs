//! crew-render: winit window + wgpu surface + glyphon text.
mod cellgrid;
mod celltext;
pub mod color;
mod gpu;
mod paperbg;
mod quads;
mod renderer;
mod roundborder;
mod scene;
mod scenecache;
mod textprep;
pub use cellgrid::CellGrid;
pub use cellgrid::CellView;
pub use paperbg::PaperBgPass;
pub use renderer::Renderer;
pub use scene::PaneScene;

/// Sorted, de-duplicated names of every installed monospace font family —
/// faces flagged monospaced plus name-matched coding fonts. GPU-free (builds
/// its own font database), so diagnostics like `crew --list-fonts` can call
/// it without a window.
pub fn list_monospace_families() -> Vec<String> {
    celltext::monospace_families(&glyphon::FontSystem::new())
}
