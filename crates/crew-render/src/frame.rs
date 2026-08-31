//! The frame path for [`crate::renderer::Renderer`]: surface acquisition,
//! uniform writes, the scene pass (paper background + cells), and — when a
//! CRT style is active — the bloom + composite reprojection. Split out of
//! `renderer.rs` to keep both files focused and under the line cap; the
//! renderer keeps ownership and this module borrows the pieces per frame.
use crate::cellgrid::CellGrid;
use crate::crtchain::CrtChain;
use crate::fadepass::FadePass;
use crate::gpu::Gpu;
use crate::paperbg::PaperBgPass;
use crate::scene::PaneScene;
use crate::solidcard::SolidCardPass;

/// Upload the scene, render, and present. Skips the frame on surface errors
/// (Outdated/Lost). `paper` is `None` with the paper texture disabled;
/// `grain` is the user knob × the theme's multiplier, precomputed upstream.
/// `fade` is the theme-crossfade strength: while `None` the finished frame is
/// snapshotted; while `Some` the held old-theme frame draws on top instead.
/// `wash_phase` is where the modern backdrop's gradient pools sit on their
/// orbit, in turns; `wash_focus` is `(centre_uv, pull)` — where that orbit is
/// centred and how far it has travelled there from the page centre.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render(
    gpu: &Gpu,
    cell_grid: &mut CellGrid,
    paper: Option<&PaperBgPass>,
    crt: &CrtChain,
    fade_pass: &mut FadePass,
    fade: Option<f32>,
    solid_card: &mut SolidCardPass,
    solid_chrome: &[[f32; 4]],
    window_opacity: f32,
    grain: f32,
    wash_phase: f32,
    wash_focus: ((f32, f32), f32),
    panes: &[PaneScene],
) {
    cell_grid.set_scene(gpu.device(), panes);
    cell_grid.prepare(
        gpu.device(),
        gpu.queue(),
        gpu.config.width,
        gpu.config.height,
    );

    let frame = match gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) => t,
        wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
        wgpu::CurrentSurfaceTexture::Outdated
        | wgpu::CurrentSurfaceTexture::Lost
        | wgpu::CurrentSurfaceTexture::Validation => {
            eprintln!("surface lost/outdated/validation — skipping frame");
            return;
        }
    };

    let view = frame.texture.create_view(&Default::default());
    let mut enc = gpu
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    // CRT on → scene renders off-screen then reprojects; off → straight to
    // the surface (the original, zero-overhead path).
    let use_crt = crt.style().is_some();
    let bg = crew_theme::theme().page_bg;
    // The page alpha IS the window opacity: it seeds the clear and the
    // paper pass, and everything drawn afterwards blends over it, so pane
    // fills and text stay solid while the bare page shows the desktop.
    let bg_f32 = crate::color::target_rgba(bg, window_opacity, gpu.format.is_srgb());
    let (w, h) = (gpu.config.width as f32, gpu.config.height as f32);

    if let Some(paper) = paper {
        // The modern family's backdrop: the gradient wash (rotated to
        // `wash_phase`, which the app only advances while a pane is busy) with
        // the dot lattice woven on top — a fine square grid pitched off the
        // text ROW height (see `cell_geometry`), so the weave scales with font
        // size and DPI. Pole colours go through the same colour-space door as
        // the page.
        let modern = crew_theme::theme().modern.map(|m| {
            // The poles are the LIVE ones — the theme's own, rotated by
            // whatever hue offset the app published this frame (see
            // crew-theme's `poleshift`). At rest that is the theme's own
            // bytes, so a still page is still a pure function of position.
            let (pole_a, pole_b) = crew_theme::poleshift::poles().unwrap_or((m.pole_a, m.pole_b));
            let c = |rgb| {
                let [r, g, b, _] = crate::color::target_rgba(rgb, 1.0, gpu.format.is_srgb());
                [r, g, b]
            };
            let (spacing, radius) = crate::paperbg::ModernPaper::cell_geometry(cell_grid.cell_h);
            crate::paperbg::ModernPaper {
                color_a: c(pole_a),
                color_b: c(pole_b),
                dots: m.dots,
                spacing,
                radius,
                // The wash lifts the very background the ink sits on, and it
                // was calibrated with only 4-16% contrast headroom over it.
                // When the OS asks for more contrast that headroom is exactly
                // what has to be given back, so the wash is scaled by the same
                // factor the spotlight is.
                wash: m.wash * crew_theme::contrast::effect_scale(),
                phase: wash_phase,
                focus: [wash_focus.0 .0, wash_focus.0 .1],
                focus_pull: wash_focus.1,
            }
        });
        paper.update_uniform(gpu.queue(), bg_f32, (w, h), 1.0, grain, modern.as_ref());
    }
    if use_crt {
        // A light page inverts the halo: see `CrtChain::update_uniforms`.
        crt.update_uniforms(gpu.queue(), w, h, !crew_theme::theme().dark);
    }

    // Where the window stops being see-through, and only while it IS sheer:
    // the card being read, plus the chrome the app holds solid (the input bar
    // and the nav — see `solidcard`). At full opacity there is nothing to hand
    // back and the pass never runs. The CRT chain carries `scene.a` through
    // its composite, so this reads the same through the tube.
    // The focused card goes FIRST: `set_rects` clamps at `MAX_SOLID_RECTS`,
    // and if the list ever outgrows it the surface being read is the last one
    // that should be dropped.
    let mut solid: Vec<[f32; 4]> = Vec::new();
    if window_opacity < 1.0 {
        solid.extend(crate::scene::focused_card_rect(
            panes,
            cell_grid.cell_w,
            cell_grid.cell_h,
        ));
        solid.extend_from_slice(solid_chrome);
    }
    solid_card.set_rects(gpu.queue(), &solid, (w, h));
    let solid_card = (!solid_card.is_empty()).then_some(&*solid_card);

    let scene_view = if use_crt { crt.scene_view() } else { &view };
    encode_scene(&mut enc, scene_view, bg_f32, paper, cell_grid, solid_card);
    if use_crt {
        crt.encode(&mut enc, &view);
    }

    // Theme crossfade, over the finished frame (post-CRT, so the old tube
    // look melts into the new one whole). Mid-fade the snapshot is frozen —
    // it must keep holding the old theme's final frame; otherwise the frame
    // that just rendered becomes the next fade's "old" frame.
    match fade {
        Some(a) => fade_pass.draw(&mut enc, gpu.queue(), &view, a),
        None => fade_pass.capture(&mut enc, &frame.texture),
    }

    gpu.queue().submit(Some(enc.finish()));
    frame.present();
}

/// Encode the scene into `scene_view`. With CRT off this IS the whole frame
/// — the original single-pass path drawing straight onto the surface.
fn encode_scene(
    enc: &mut wgpu::CommandEncoder,
    scene_view: &wgpu::TextureView,
    bg_f32: [f32; 4],
    paper: Option<&PaperBgPass>,
    cell_grid: &CellGrid,
    solid_card: Option<&SolidCardPass>,
) {
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
                    // Carries the window opacity (see above) — a hard 1.0
                    // here would make the window opaque no matter what the
                    // paper pass writes, and with the paper texture off
                    // there IS no paper pass.
                    a: bg_f32[3] as f64,
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
    // Last, and only in the alpha channel: everything above has finished
    // writing colour, so solidifying the focused card cannot change any of it
    // (see [`crate::solidcard`]).
    if let Some(solid) = solid_card {
        solid.draw(&mut pass);
    }
}
