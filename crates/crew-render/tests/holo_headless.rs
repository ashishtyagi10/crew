//! Headless GPU test for neon's backdrop (`holo.wgsl`): that it compiles, that
//! it lays a grid of light over the page on the one theme that has it and
//! nothing on any other, and that only its sparks move with the clock.
//!
//! On macOS with Metal this renders for real; in GPU-less CI it skips.
use crew_render::holo::{set_phase, HoloLayer};
use crew_theme::ThemeId;

const SIZE: u32 = 128;
/// Row stride of the readback buffer (COPY_BYTES_PER_ROW_ALIGNMENT).
const STRIDE: usize = 512;
/// A text cell: the grid's squares are four by two of them, 32 px.
const CELL: (f32, f32) = (8.0, 16.0);

fn render(device: &wgpu::Device, queue: &wgpu::Queue, phase: f32) -> Vec<u8> {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut layer = HoloLayer::new(device, format);
    set_phase(phase);
    layer.prepare(queue, (SIZE as f32, SIZE as f32), CELL);
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("holo_test_tex"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
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
        label: Some("holo_readback"),
        size: (STRIDE as u32 * SIZE) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        // The page, premultiplied at a sheer window's opacity, as the frame
        // clears it.
        let (r, g, b) = crew_theme::theme().page_bg;
        let a = 0.84;
        let c = |v: u8| f64::from(v) / 255.0 * a;
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("holo_test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: c(r),
                        g: c(g),
                        b: c(b),
                        a,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        layer.draw(&mut rp);
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
                bytes_per_row: Some(STRIDE as u32),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
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
    let data = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    data
}

/// The light at `(x, y)`: the sum of its premultiplied channels.
fn light(buf: &[u8], x: usize, y: usize) -> u32 {
    let o = y * STRIDE + x * 4;
    u32::from(buf[o]) + u32::from(buf[o + 1]) + u32::from(buf[o + 2])
}

#[test]
fn the_grid_lights_neon_alone_and_only_its_sparks_move() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("holo: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    // Any other theme: the pass draws nothing, and the page is the clear.
    crew_theme::set_theme(ThemeId::GlassNight);
    let plain = render(&device, &queue, 0.0);
    let page = light(&plain, 5, 5);
    assert!(
        (0..SIZE as usize).all(|y| (0..SIZE as usize).all(|x| light(&plain, x, y) == page)),
        "no grid off neon"
    );

    crew_theme::set_theme(ThemeId::NeonDark);
    let at_rest = render(&device, &queue, 0.0);
    // A line runs down x = 64 (the squares are 32 px); the middle of a
    // square lies 16 px from any line.
    let (line, mid) = (light(&at_rest, 64, 40), light(&at_rest, 48, 48));
    assert!(line > mid + 6, "the line is light: {line} over {mid}");
    let changed = |a: &[u8], b: &[u8]| {
        (0..SIZE as usize)
            .flat_map(|y| (0..SIZE as usize).map(move |x| (x, y)))
            .filter(|&(x, y)| light(a, x, y) != light(b, x, y))
            .count()
    };
    let again = render(&device, &queue, 0.0);
    assert_eq!(
        changed(&at_rest, &again),
        0,
        "a held clock draws the same frame"
    );
    let later = render(&device, &queue, 0.3);
    let moved = changed(&at_rest, &later);
    let all = (SIZE * SIZE) as usize;
    assert!(
        moved > 0 && moved < all / 4,
        "the sparks move and the grid stays: {moved} of {all} px changed"
    );
    assert_eq!(light(&later, 64, 40), line, "a line under no spark holds");
}
