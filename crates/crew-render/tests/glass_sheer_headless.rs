//! Liquid glass on a see-through window (2026-10-08, the user: "glass theme
//! is not glassy enough, I can't see the background"): the slab gives up
//! alpha so the desktop shows through it — and keeps its colour while it
//! does. The blend would mix the page under the card back in by the alpha
//! given up, so the shader takes that share out first; this renders a slab
//! solid and see-through over the same page and holds the two to the same
//! colour, with the see-through one's alpha where the theme says. The scene
//! is stored premultiplied, as the frame stores it — including a slab
//! CLEARER than its own frost, which straight colour could not draw.
use crew_render::{GlassCard, GlassLayer};

const SIZE: u32 = 64;
const STRIDE: usize = 256;
const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The page: a warm pastel, at the window's opacity.
const PAGE: [u8; 4] = [236, 200, 214, 255];
const WINDOW: f64 = 0.45;

fn slab(see: f32, frost: f32) -> GlassCard {
    GlassCard {
        x: 12.0,
        y: 12.0,
        w: 40.0,
        h: 40.0,
        radius: 8.0,
        alpha_top: frost,
        alpha_bottom: frost,
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
        // The body is the page, frosted white.
        see,
        liquid: true,
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

/// The page as the glass takes it: straight, its alpha the window's opacity.
fn page() -> [f32; 4] {
    let c = |v: u8| f32::from(v) / 255.0;
    [c(PAGE[0]), c(PAGE[1]), c(PAGE[2]), WINDOW as f32]
}

/// The slab's centre pixel, drawn over the page at the window's opacity.
fn centre(device: &wgpu::Device, queue: &wgpu::Queue, see: f32, frost: f32) -> [u8; 4] {
    at(device, queue, slab(see, frost), (32, 32))
}

/// The pixel at `(x, y)` with `card` drawn over the page at the window's
/// opacity.
fn at(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    card: GlassCard,
    (x, y): (usize, usize),
) -> [u8; 4] {
    let mut layer = GlassLayer::new(device, FMT);
    layer.set_page(queue, page());
    layer.set_cards(device, &[card]);
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
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("glass_sheer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(crew_render::color::premultiplied([
                        f32::from(PAGE[0]) / 255.0,
                        f32::from(PAGE[1]) / 255.0,
                        f32::from(PAGE[2]) / 255.0,
                        WINDOW as f32,
                    ])),
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
    let o = y * STRIDE + x * 4;
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
    // Clearer than the frost (0.7 of the desktop through a 0.5 frost) as well
    // as frostier: the first is the one straight colour clamped grey.
    for (see, frost) in [(0.35_f32, 0.5_f32), (0.7, 0.5)] {
        let solid = centre(&device, &queue, 0.0, frost);
        let sheer = centre(&device, &queue, see, frost);
        eprintln!("glass_sheer_headless: see {see}: solid {solid:?}, see-through {sheer:?}");
        // Solid glass covers the desktop whole, as it always did.
        assert_eq!(solid[3], 255, "a solid slab let the desktop through");
        // See-through: the slab's own alpha over the page's ("over").
        let body = 1.0 - see;
        let want = ((body + WINDOW as f32 * see) * 255.0).round() as u8;
        assert!(
            sheer[3].abs_diff(want) <= 1,
            "see {see}: alpha {} not {want}",
            sheer[3]
        );
        // …and it is the same glass: the frost, not the page, under the text.
        for c in 0..3 {
            let own = (f32::from(sheer[c]) * 255.0 / f32::from(sheer[3])).round() as u8;
            assert!(
                own.abs_diff(solid[c]) <= 2,
                "see {see} channel {c}: {own} see-through vs {} solid",
                solid[c]
            );
        }
        // The frost is really there (the test is not comparing page to page).
        assert!(solid[2] > PAGE[2] + 10, "no frost: {solid:?}");
    }
}

/// A legend stands on the rule, half of it outside the sheet. On
/// see-through glass that half sat on the desktop — dark words on a dark
/// desktop vanished — so the body reaches out behind the words as a veil:
/// there and only there, and only when the window is see-through.
#[test]
fn legend_veil_headless() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("legend_veil_headless: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");
    // A legend over x 8..24 of the card (x 20..36 on screen); the card's top
    // edge is at y 12, the legend's row reaching 6 px above it.
    let card = |see: f32| {
        let mut c = slab(see, 0.5);
        c.notch.top[0] = [8.0, 24.0];
        c.notch.depth = 6.0;
        c
    };
    // The page's alpha as the readback stores it.
    let page = (255.0 * WINDOW).round() as u8;
    let behind = at(&device, &queue, card(0.6), (28, 9));
    let beside = at(&device, &queue, card(0.6), (46, 9));
    let solid = at(&device, &queue, card(0.0), (28, 9));
    eprintln!("legend_veil_headless: behind {behind:?} beside {beside:?} opaque {solid:?}");
    assert!(
        behind[3] > page + 40,
        "no veil behind the legend: {behind:?} over a page at {page}"
    );
    assert!(
        beside[3] <= page + 2,
        "the veil spread past the legend: {beside:?}"
    );
    assert!(
        solid[3] <= page + 2,
        "an opaque window grew a tab: {solid:?}"
    );
}
