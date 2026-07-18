//! Frame encoding for [`crate::renderer::Renderer`]: the scene pass (paper
//! background + cells) and, when CRT is on, the reprojection pass. Split out of
//! `renderer.rs` to keep both files focused and under the line cap. Every `draw`
//! here takes `&self`, so the passes borrow disjoint renderer fields.
use crate::cellgrid::CellGrid;
use crate::crt::CrtPass;
use crate::paperbg::PaperBgPass;

/// Encode the frame. The scene draws into `scene_view`; when `use_crt`, the CRT
/// pass then reprojects that off-screen scene onto `surface_view`. With CRT off
/// the caller passes `scene_view == surface_view` and no second pass runs — the
/// original single-pass path. Uniforms are written by the caller beforehand.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    enc: &mut wgpu::CommandEncoder,
    surface_view: &wgpu::TextureView,
    scene_view: &wgpu::TextureView,
    use_crt: bool,
    bg_f32: [f32; 4],
    paper: Option<&PaperBgPass>,
    cell_grid: &CellGrid,
    crt: &CrtPass,
) {
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("crew frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: scene_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg_f32[0] as f64,
                        g: bg_f32[1] as f64,
                        b: bg_f32[2] as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(paper) = paper {
            paper.draw(&mut pass);
        }
        cell_grid.draw(&mut pass);
    }

    if use_crt {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("crt"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // The tube fills the surface; the bezel is shader-drawn, so
                    // this clear is only a safety net.
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        crt.draw(&mut pass);
    }
}
