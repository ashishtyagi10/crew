//! Headless GPU check for pictures drawn as textures (`crew_render::Picture`,
//! the reader loop, round 3): a four-colour picture drawn into a box lands
//! with each quarter its own colour, nothing outside the box, and the part
//! of it past its clip not drawn at all. Skips where there is no GPU adapter.
use std::sync::Arc;

use crew_render::{CellGrid, PaneScene, Picture, Pixels};

const W: u32 = 64;
const H: u32 = 64;

fn px(buf: &[u8], x: usize, y: usize) -> (u8, u8, u8, u8) {
    let i = y * W as usize * 4 + x * 4;
    (buf[i], buf[i + 1], buf[i + 2], buf[i + 3])
}

/// A 2×2 picture: red, green / blue, white, filling a 40×40 px box at
/// (8, 8), clipped `clip_y` px down the pane.
fn scene(cw: f32, ch: f32, clip_y: f32) -> Vec<PaneScene> {
    let rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, //
        0, 0, 255, 255, 255, 255, 255, 255,
    ];
    vec![PaneScene {
        x: 0.0,
        y: 0.0,
        w: W as f32,
        h: H as f32,
        pictures: vec![Picture {
            pixels: Arc::new(Pixels::new(2, 2, rgba)),
            x: 8.0 / cw,
            y: 8.0 / ch,
            w: 40.0 / cw,
            h: 40.0 / ch,
            clip: [0.0, 0.0, W as f32 / cw, clip_y / ch],
        }],
        ..Default::default()
    }]
}

fn render(device: &wgpu::Device, queue: &wgpu::Queue, scenes: &[PaneScene]) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut grid = CellGrid::new(device, queue, format, 14.0);
    grid.set_scene(device, scenes);
    grid.prepare(device, queue, W, H);

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("picture_test"),
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
            label: Some("picture_pass"),
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
fn a_picture_is_drawn_as_a_texture_in_its_box_and_cut_at_its_clip() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("no GPU adapter, skipping");
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        eprintln!("no device, skipping");
        return;
    };
    let grid = CellGrid::new(&device, &queue, wgpu::TextureFormat::Rgba8Unorm, 14.0);
    let (cw, ch) = grid.cell_size();
    let whole = render(&device, &queue, &scene(cw, ch, H as f32));
    let red = px(&whole, 12, 12);
    let white = px(&whole, 44, 44);
    assert!(
        red.0 > 200 && red.1 < 60 && red.2 < 60,
        "top left is red: {red:?}"
    );
    assert!(
        white.0 > 200 && white.1 > 200 && white.2 > 200,
        "bottom right white: {white:?}"
    );
    assert_eq!(px(&whole, 4, 4).3, 0, "nothing outside the box");
    assert_eq!(px(&whole, 56, 56).3, 0);
    let cut = render(&device, &queue, &scene(cw, ch, 28.0));
    assert!(px(&cut, 12, 12).0 > 200, "above the clip it shows");
    assert_eq!(px(&cut, 44, 44).3, 0, "past the clip nothing is drawn");
}
