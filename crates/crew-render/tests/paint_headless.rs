//! Headless GPU check for the paint layer's blending.
//!
//! The quad pipeline drew with `BlendState::REPLACE` for as long as everything
//! it drew was opaque. Translucent chart fills are the first thing that can
//! tell the difference — and the difference is invisible in unit tests, which
//! see the alpha reach the `Quad` and stop there. This renders a translucent
//! rectangle over an opaque cell background through the real `CellGrid` and
//! reads the pixel back: under `REPLACE` it would come out pure paint colour.
//!
//! Skips (rather than fails) where there is no GPU adapter.
use crew_render::{CellGrid, CellView, Paint, PaneScene};

const W: u32 = 64;
const H: u32 = 64;

fn px(buf: &[u8], x: usize, y: usize) -> (u8, u8, u8) {
    let i = y * W as usize * 4 + x * 4;
    (buf[i], buf[i + 1], buf[i + 2])
}

/// One pane: a black cell background across the whole thing, with a white
/// rectangle painted over the left half at `alpha`.
fn scene(alpha: f32, cols: u16, rows: u16) -> Vec<PaneScene> {
    let cells = (0..rows)
        .flat_map(|row| {
            (0..cols).map(move |col| CellView {
                col,
                row,
                c: ' ',
                fg: (255, 255, 255),
                bg: (0, 0, 0),
                ..Default::default()
            })
        })
        .collect();
    vec![PaneScene {
        cells,
        x: 0.0,
        y: 0.0,
        w: W as f32,
        h: H as f32,
        focused: false,
        bordered: false,
        glass: false,
        scan: -1.0,
        overlay: false,
        paint: vec![Paint::solid(
            0.0,
            0.0,
            f32::from(cols) / 2.0,
            f32::from(rows),
            (255, 255, 255),
        )
        .at(alpha)],
    }]
}

fn render(device: &wgpu::Device, queue: &wgpu::Queue, alpha: f32) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut grid = CellGrid::new(device, queue, format, 14.0);
    let (cw, ch) = grid.cell_size();
    let cols = (W as f32 / cw).floor() as u16;
    let rows = (H as f32 / ch).floor() as u16;
    grid.set_scene(device, &scene(alpha, cols.max(1), rows.max(1)));
    grid.prepare(device, queue, W, H);

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("paint_test"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(W * H * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("paint_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        grid.draw(&mut pass);
    }
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(W * 4),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));
    let wait = || wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    };
    device.poll(wait()).expect("poll failed");
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wait()).expect("poll failed");
    let out = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    out
}

#[test]
fn translucent_paint_blends_with_what_is_under_it() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("translucent_paint_blends_with_what_is_under_it: no GPU adapter, skipping");
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        eprintln!("no device, skipping");
        return;
    };

    // Half-alpha white over black: mid-grey. Under REPLACE this pixel would
    // read 255 — the whole point of the layer (a chart fill you can see the
    // page through) would be a solid slab instead.
    let half = render(&device, &queue, 0.5);
    let (r, g, b) = px(&half, 4, 4);
    assert!(
        (96..=160).contains(&r) && r == g && g == b,
        "blended grey, got ({r}, {g}, {b})"
    );

    // Opaque paint still covers completely, and the half of the pane it does
    // not reach keeps the cell background it was given.
    let full = render(&device, &queue, 1.0);
    assert_eq!(px(&full, 4, 4), (255, 255, 255));
    assert_eq!(px(&full, W as usize - 4, 4), (0, 0, 0));
}
