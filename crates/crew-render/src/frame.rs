//! The frame path for [`crate::renderer::Renderer`]: surface acquisition,
//! uniform writes, the scene pass (page + cells), and — when a
//! CRT style is active — the bloom + composite reprojection. Split out of
//! `renderer.rs` to keep both files focused and under the line cap; the
//! renderer keeps ownership and this module borrows the pieces per frame.
use crate::cellgrid::CellGrid;
use crate::crtchain::CrtChain;
use crate::fadepass::FadePass;
use crate::gpu::Gpu;
use crate::scene::PaneScene;
use crate::solidcard::SolidCardPass;

/// Upload the scene, render, and present. Skips the frame on surface errors
/// (Outdated/Lost). `fade` is the theme-crossfade strength: while `None` the
/// finished frame is snapshotted; while `Some` the held old-theme frame draws
/// on top instead.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render(
    gpu: &Gpu,
    cell_grid: &mut CellGrid,
    crt: &CrtChain,
    fade_pass: &mut FadePass,
    fade: Option<f32>,
    solid_card: &mut SolidCardPass,
    solid_chrome: &[[f32; 4]],
    window_opacity: f32,
    panes: &[PaneScene],
) {
    // The surface is asked FIRST, before a single byte is uploaded.
    //
    // Everything below this line allocates on the GPU — four instance buffers
    // in `set_scene`, glyphon's vertex upload in `prepare` — and hands last
    // frame's buffers to wgpu's destruction queue. That queue is only drained
    // when the device is maintained, and the only thing that maintains it on
    // this path is the `submit` at the bottom. So a frame that allocated and
    // then bailed out before submitting freed NOTHING.
    //
    // Which is exactly what a sleeping display does: the surface answers
    // Occluded or Timeout for hours while the panes below keep streaming and
    // asking for redraws, and every one of those frames used to allocate a
    // fresh set of buffers that nothing would ever reclaim. One night of that
    // took 75 GB and the machine with it.
    let frame = match gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) => t,
        wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            return drain(gpu)
        }
        wgpu::CurrentSurfaceTexture::Outdated
        | wgpu::CurrentSurfaceTexture::Lost
        | wgpu::CurrentSurfaceTexture::Validation => {
            eprintln!("surface lost/outdated/validation — skipping frame");
            return drain(gpu);
        }
    };

    cell_grid.set_scene(gpu.device(), panes);
    cell_grid.prepare(
        gpu.device(),
        gpu.queue(),
        gpu.config.width,
        gpu.config.height,
    );

    let view = frame.texture.create_view(&Default::default());
    let mut enc = gpu
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    // CRT on → scene renders off-screen then reprojects; off → straight to
    // the surface (the original, zero-overhead path). A see-through window
    // takes the chain too: it hands the premultiplied scene over
    // (`CrtChain::set_sheer`).
    let use_crt = crt.active();
    let bg = crew_theme::theme().page_bg;
    // The page alpha IS the window opacity: it seeds the clear, and
    // everything drawn afterwards blends over it, so pane
    // fills and text stay solid while the bare page shows the desktop.
    let bg_f32 = crate::color::target_rgba(bg, window_opacity, gpu.format.is_srgb());
    let (w, h) = (gpu.config.width as f32, gpu.config.height as f32);

    if use_crt {
        // A light page inverts the halo: see `CrtChain::update_uniforms`.
        crt.update_uniforms(gpu.queue(), w, h, !crew_theme::theme().dark);
    }

    // Where the window stops being see-through, and only while it IS sheer:
    // the overlays the app holds solid (see `solidcard`). The panes, the nav
    // and the input bar are NOT on the list: they are what the opacity is
    // for, frosted by the window server behind them. At full
    // opacity there is nothing to hand back and the pass never runs. The CRT
    // chain carries `scene.a` through its composite, so this reads the same
    // through the tube.
    let solid: &[[f32; 4]] = if window_opacity < 1.0 {
        solid_chrome
    } else {
        &[]
    };
    solid_card.set_rects(gpu.queue(), solid, (w, h));
    let solid_card = (!solid_card.is_empty()).then_some(&*solid_card);

    // The glass smokes the page and, on a see-through window, gives the
    // page's share of the desktop back (`GlassLayer::set_page`).
    cell_grid.set_page(gpu.queue(), bg_f32);
    let scene_view = if use_crt { crt.scene_view() } else { &view };
    encode_scene(&mut enc, scene_view, bg_f32, cell_grid, solid_card);
    if use_crt {
        crt.encode(&mut enc, &view);
    }

    // Theme crossfade, over the finished frame (post-CRT, so the old tube
    // look melts into the new one whole). Mid-fade the snapshot is frozen —
    // it must keep holding the old theme's final frame; otherwise the frame
    // that just rendered becomes the next fade's "old" frame.
    match fade {
        Some(a) => fade_pass.draw(&mut enc, gpu.queue(), &view, a),
        None => fade_pass.capture_as(
            &mut enc,
            &frame.texture,
            crate::crtchain::premultiplies(window_opacity),
        ),
    }

    gpu.queue().submit(Some(enc.finish()));
    frame.present();
}

/// Maintain the device on a path that will not submit.
///
/// wgpu reclaims a dropped buffer's memory when the device is maintained, and
/// `Queue::submit` is what normally does it. A frame that returns early never
/// submits, so it has to say so itself — otherwise the last frame's buffers
/// (and every frame's before it) sit in the destruction queue for as long as
/// the surface stays unavailable. Non-blocking: a single check, no wait.
fn drain(gpu: &Gpu) {
    let _ = gpu.device().poll(wgpu::PollType::Poll);
}

/// Encode the scene into `scene_view`. With CRT off this IS the whole frame
/// — the original single-pass path drawing straight onto the surface.
fn encode_scene(
    enc: &mut wgpu::CommandEncoder,
    scene_view: &wgpu::TextureView,
    bg_f32: [f32; 4],
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
                // Carries the window opacity (see above) — a hard 1.0 here
                // would make the window opaque. Premultiplied, as the whole
                // scene is stored.
                load: wgpu::LoadOp::Clear(crate::color::premultiplied(bg_f32)),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    cell_grid.draw(&mut pass);
    // Last, and only in the alpha channel: everything above has finished
    // writing colour, so solidifying the focused card cannot change any of it
    // (see [`crate::solidcard`]).
    if let Some(solid) = solid_card {
        solid.draw(&mut pass);
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
