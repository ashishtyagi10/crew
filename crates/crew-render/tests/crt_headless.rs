//! Headless GPU integration test for the CRT post-process pass. Renders a
//! known source texture through `CrtPass` and reads pixels back to prove the
//! tube physics actually happen: the flat panel fills edge-to-edge (no barrel
//! warp, no black bezel), scanlines darken alternating rows, phosphor glow
//! bleeds a bright block into its dark neighbours, and `flicker = 0` is
//! byte-for-byte static.
//!
//! On macOS/Metal this runs the real GPU render; on GPU-less CI it skips.
use crew_render::CrtPass;

const N: usize = 64;
const STRIDE: usize = 256; // N * 4, already a COPY_BYTES_PER_ROW_ALIGNMENT multiple

fn r_at(buf: &[u8], x: usize, y: usize) -> u8 {
    buf[y * STRIDE + x * 4]
}

/// A source texture filled from `fill(x, y) -> [r,g,b,a]`.
fn source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    fill: impl Fn(usize, usize) -> [u8; 4],
) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("crt_src"),
        size: wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut data = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            data[(y * N + x) * 4..(y * N + x) * 4 + 4].copy_from_slice(&fill(x, y));
        }
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(N as u32 * 4),
            rows_per_image: Some(N as u32),
        },
        wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
    );
    tex.create_view(&Default::default())
}

fn render(device: &wgpu::Device, queue: &wgpu::Queue, crt: &CrtPass) -> Vec<u8> {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("crt_out"),
        size: wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("crt_readback"),
        size: (N * N * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&Default::default());
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("crt_test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        crt.draw(&mut rp);
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
                rows_per_image: Some(N as u32),
            },
        },
        wgpu::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));
    let wait = || {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll failed");
    };
    wait();
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    wait();
    let data = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    data
}

#[test]
fn crt_headless() {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("crt_headless: no GPU adapter, skipping");
            return;
        }
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    let mut crt = CrtPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);

    // --- Case 1: a solid mid-gray field → flat geometry + scanlines ---
    // Mid-gray (not white) so the scanline darkening stays visible: on a
    // saturated white field the phosphor glow would push every row past 1.0 and
    // both light and dark lines would clamp to 255, hiding the effect.
    let gray = source(&device, &queue, |_, _| [160, 160, 160, 255]);
    crt.set_source(&device, &gray);
    crt.update_uniform(&queue, N as f32, N as f32, 0.0, 0.0);
    let px = render(&device, &queue, &crt);

    // Flat geometry: with curvature 0 the image maps 1:1 and fills the panel
    // edge-to-edge, so every corner is lit — there is no bezel to black out.
    for (x, y) in [(0, 0), (N - 1, 0), (0, N - 1), (N - 1, N - 1)] {
        assert!(
            r_at(&px, x, y) > 40,
            "corner ({x},{y}) should be lit on a flat panel, got {}",
            r_at(&px, x, y)
        );
    }
    // The center is lit too.
    assert!(
        r_at(&px, N / 2, N / 2) > 40,
        "center should be lit, got {}",
        r_at(&px, N / 2, N / 2)
    );
    // Scanlines: the 2-pixel cosine darkens every other line, so ADJACENT rows
    // differ sharply. Adjacent-row delta isolates the scanline from the slow
    // corner-darkening falloff (which barely changes between neighbours).
    let col = N / 2;
    let max_adjacent = (24..40)
        .map(|y| (r_at(&px, col, y) as i32 - r_at(&px, col, y + 1) as i32).abs())
        .max()
        .unwrap_or(0);
    assert!(
        max_adjacent >= 12,
        "scanlines should make adjacent center rows differ, max delta was {max_adjacent}"
    );

    // --- Case 2: a bright block on black → phosphor glow bleed ---
    let block = source(&device, &queue, |x, y| {
        let hot = (28..36).contains(&x) && (28..36).contains(&y);
        if hot {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 255]
        }
    });
    crt.set_source(&device, &block);
    crt.update_uniform(&queue, N as f32, N as f32, 0.0, 0.0);
    let g = render(&device, &queue, &crt);
    // A pixel just outside the block is black in the source but the glow taps
    // reach into the block, so it must pick up some light.
    assert!(
        r_at(&g, 37, 32) > 0,
        "glow should bleed past the block edge, got {}",
        r_at(&g, 37, 32)
    );

    // --- Case 3: flicker = 0 is perfectly static (deterministic) ---
    crt.set_source(&device, &gray);
    crt.update_uniform(&queue, N as f32, N as f32, 123.0, 0.0);
    let a = render(&device, &queue, &crt);
    crt.update_uniform(&queue, N as f32, N as f32, 456.0, 0.0);
    let b = render(&device, &queue, &crt);
    assert_eq!(a, b, "flicker=0 must be static regardless of time");

    eprintln!("crt_headless: flat geometry, scanlines, glow, static-flicker all verified");
}
