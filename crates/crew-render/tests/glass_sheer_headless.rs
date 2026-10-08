//! Liquid glass on a see-through window (2026-10-08, the user: "glass theme
//! is not glassy enough, I can't see the background"): the slab gives up
//! alpha so the desktop shows through it — and keeps its colour while it
//! does. The blend would mix the page under the card back in by the alpha
//! given up, so the shader takes that share out first; this renders a slab
//! solid and see-through over the same page and holds the two to the same
//! colour, with the see-through one's alpha where the theme says.
use crew_render::{GlassCard, GlassLayer};

const SIZE: u32 = 64;
const STRIDE: usize = 256;
const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The page: a warm pastel, at the window's opacity.
const PAGE: [u8; 4] = [236, 200, 214, 255];
const WINDOW: f64 = 0.45;

fn slab(see: f32) -> GlassCard {
    GlassCard {
        x: 12.0,
        y: 12.0,
        w: 40.0,
        h: 40.0,
        radius: 8.0,
        alpha_top: 0.5,
        alpha_bottom: 0.5,
        noise: 0.0,
        tint: [1.0, 1.0, 1.0, 1.0],
        highlight: [1.0, 1.0, 1.0, 1.0],
        highlight_alpha: 0.0,
        shadow_alpha: 0.0,
        scan: -1.0,
        edge_glow: 0.0,
        gloss: 0.0,
        glow: 0.0,
        etch: 0.0,
        lift: 0.0,
        glint: -1.0,
        notch: Default::default(),
        // A lens with no bend or blur: the body is the page, frosted white.
        lens: [0.0, 1.0, 0.0, 0.0, 0.0, 1.0, see, 1.0],
    }
}

fn texture(device: &wgpu::Device, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glass_sheer"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FMT,
        usage,
        view_formats: &[],
    })
}

/// The slab's centre pixel, drawn over the page at the window's opacity.
fn centre(device: &wgpu::Device, queue: &wgpu::Queue, see: f32) -> [u8; 4] {
    let behind = texture(
        device,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &behind,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &PAGE.repeat((SIZE * SIZE) as usize),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE * 4),
            rows_per_image: Some(SIZE),
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    let mut layer = GlassLayer::new(device, FMT);
    layer.set_behind(device, &behind.create_view(&Default::default()));
    layer.set_cards(device, &[slab(see)]);
    layer.set_view(queue, SIZE as f32, SIZE as f32, (0.0, 0.0));
    let out = texture(
        device,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("glass_sheer_readback"),
        size: (SIZE * SIZE * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = out.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&Default::default());
    {
        let c = |v: u8| f64::from(v) / 255.0;
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass_sheer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: c(PAGE[0]),
                        g: c(PAGE[1]),
                        b: c(PAGE[2]),
                        a: WINDOW,
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
            texture: &out,
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
    let o = 32 * STRIDE + 32 * 4;
    [data[o], data[o + 1], data[o + 2], data[o + 3]]
}

#[test]
fn glass_sheer_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("glass_sheer_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    let solid = centre(&device, &queue, 0.0);
    let see = 0.35;
    let sheer = centre(&device, &queue, see);
    eprintln!("glass_sheer_headless: solid {solid:?}, see-through {sheer:?}");
    // Solid glass covers the desktop whole, as it always did.
    assert_eq!(solid[3], 255, "a solid slab let the desktop through");
    // See-through: the slab's own alpha over the page's ("over").
    let body = 1.0 - see;
    let want = ((body + WINDOW as f32 * (1.0 - body)) * 255.0).round() as u8;
    assert!(
        sheer[3].abs_diff(want) <= 1,
        "alpha {} not {want}",
        sheer[3]
    );
    // …and it is the same glass: the frost, not the page, under the text.
    for c in 0..3 {
        assert!(
            sheer[c].abs_diff(solid[c]) <= 2,
            "channel {c}: {} see-through vs {} solid",
            sheer[c],
            solid[c]
        );
    }
    // The frost is really there (the test is not comparing page to page).
    assert!(solid[2] > PAGE[2] + 10, "no frost: {solid:?}");
}
