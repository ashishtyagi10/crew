//! Headless GPU integration test for the theme-crossfade pass: captures a
//! solid "old frame", then draws it back over a differently-colored "new
//! frame" at full and partial fade, asserting the actual blended pixels.
//!
//! On macOS with Metal this finds an adapter and runs the real GPU render;
//! in GPU-less CI it gracefully skips instead of failing.
use crew_render::FadePass;

const SIZE: u32 = 64;

fn make_texture(device: &wgpu::Device, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("fade_test"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    })
}

/// Clear `tex` to `color` on the GPU.
fn clear_to(device: &wgpu::Device, queue: &wgpu::Queue, tex: &wgpu::Texture, color: wgpu::Color) {
    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    queue.submit(Some(enc.finish()));
}

/// Read back the RGBA of the centre pixel.
fn centre_pixel(device: &wgpu::Device, queue: &wgpu::Queue, tex: &wgpu::Texture) -> [u8; 4] {
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (SIZE * SIZE * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    enc.copy_texture_to_buffer(
        tex.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
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
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll failed");
    let data = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    let i = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;
    [data[i], data[i + 1], data[i + 2], data[i + 3]]
}

#[test]
fn fade_headless_crossfades_the_captured_frame() {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("fade_headless: no GPU adapter, skipping");
            return;
        }
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    // Also validates fade.wgsl via naga.
    let mut fade = FadePass::new(&device, wgpu::TextureFormat::Rgba8Unorm, SIZE, SIZE, true);
    assert!(!fade.ready(), "no frame captured yet");

    // "Old frame": solid red, captured into the snapshot.
    let old = make_texture(
        &device,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    clear_to(&device, &queue, &old, wgpu::Color::RED);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    fade.capture(&mut enc, &old);
    queue.submit(Some(enc.finish()));
    assert!(fade.ready(), "capture must arm the snapshot");

    // "New frame": solid blue surface the fade draws over.
    let surface = make_texture(
        &device,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let surface_view = surface.create_view(&Default::default());

    // Full fade → the old frame fully covers the new one.
    clear_to(&device, &queue, &surface, wgpu::Color::BLUE);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    fade.draw(&mut enc, &queue, &surface_view, 1.0);
    queue.submit(Some(enc.finish()));
    let px = centre_pixel(&device, &queue, &surface);
    assert!(
        px[0] >= 254 && px[2] <= 1,
        "fade 1.0 must show the old (red) frame, got {px:?}"
    );

    // Quarter fade → 25% old red over new blue.
    clear_to(&device, &queue, &surface, wgpu::Color::BLUE);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    fade.draw(&mut enc, &queue, &surface_view, 0.25);
    queue.submit(Some(enc.finish()));
    let px = centre_pixel(&device, &queue, &surface);
    let (r, b) = (px[0] as i32, px[2] as i32);
    assert!(
        (r - 64).abs() <= 2 && (b - 191).abs() <= 2,
        "fade 0.25 must blend 25% red over blue, got {px:?}"
    );

    // A resize drops the held frame (mid-fade resize hard-cuts, never
    // stretches a stale snapshot).
    fade.resize(&device, SIZE * 2, SIZE * 2);
    assert!(!fade.ready(), "resize must invalidate the snapshot");
}

/// See-through glass leaves its frame PREMULTIPLIED, and the glass themes
/// rotate every ten minutes through this crossfade. Drawn back as straight
/// colour, the held frame was darkened a second time by its own alpha — a
/// dim ghost of the old glass for the length of every fade.
#[test]
fn fade_headless_hands_a_premultiplied_frame_back_as_it_was() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("fade_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let mut fade = FadePass::new(&device, wgpu::TextureFormat::Rgba8Unorm, SIZE, SIZE, true);
    // Glass at half alpha, premultiplied: a light grey worth 0.8 shows 0.4.
    let old = make_texture(
        &device,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let glass = wgpu::Color {
        r: 0.4,
        g: 0.4,
        b: 0.4,
        a: 0.5,
    };
    clear_to(&device, &queue, &old, glass);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    fade.capture_as(&mut enc, &old, true);
    queue.submit(Some(enc.finish()));

    let surface = make_texture(
        &device,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let surface_view = surface.create_view(&Default::default());
    clear_to(&device, &queue, &surface, wgpu::Color::TRANSPARENT);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    fade.draw(&mut enc, &queue, &surface_view, 1.0);
    queue.submit(Some(enc.finish()));
    let px = centre_pixel(&device, &queue, &surface);
    assert!(
        (i32::from(px[0]) - 102).abs() <= 2 && (i32::from(px[3]) - 128).abs() <= 2,
        "the held glass came back darkened: {px:?} (want ~[102, 102, 102, 128])"
    );
}
