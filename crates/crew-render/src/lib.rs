//! crew-render: winit window + wgpu surface + glyphon text.
mod bloom;
mod cellgrid;
mod celltext;
pub mod color;
mod crt;
mod crtchain;
mod embedfont;
mod fadepass;
mod fontlist;
mod fontverify;
mod frame;
mod glass;
mod gpu;
mod paperbg;
mod postfx;
mod prewarm;
mod quads;
mod renderer;
mod roundborder;
mod scene;
mod scenecache;
mod scenetarget;
mod smoothing;
mod textprep;
pub use cellgrid::CellGrid;
pub use cellgrid::CellView;
pub use crtchain::CrtChain;
pub use embedfont::font_system;
pub use fadepass::FadePass;
pub use glass::{GlassCard, GlassLayer};
pub use paperbg::{ModernPaper, PaperBgPass};
pub use renderer::Renderer;
pub use scene::PaneScene;
pub use smoothing::DEFAULT_SMOOTH;

/// Sorted, de-duplicated names of every installed monospace font family —
/// flagged/name-matched candidates verified to render fixed-pitch Latin (see
/// [`fontlist`]). GPU-free (builds its own font database), so diagnostics
/// like `crew --list-fonts` can call it without a window.
pub fn list_monospace_families() -> Vec<String> {
    // No grid here, so screen at a nominal 16px cell — `snaps_to_cells` is
    // scale-invariant for a fixed-pitch face (its advance is a constant
    // fraction of the size), so this reports what the app will accept.
    const SIZE: f32 = 16.0;
    let cell_w = (SIZE * celltext::CELL_W_RATIO).round();
    let mut fs = embedfont::font_system();
    let mut fams = fontlist::monospace_families(&mut fs);
    fams.retain(|f| fontverify::snaps_to_cells(&mut fs, f, SIZE, cell_w, 600));
    fams
}
