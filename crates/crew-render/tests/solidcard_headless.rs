//! Headless GPU test for the focused card's solidity pass.
//!
//! The claim `solidcard.wgsl` makes is unusual enough to be worth pixels: it
//! writes ONLY the alpha channel, so the rects it solidifies keep every colour
//! that was already there — the gradient wash, the lattice, the grain, the
//! text. A `write_mask` typo would paint that card black and no unit test
//! could tell; a pass that never ran would look identical to a working one
//! from Rust (which is exactly how v0.7.0 shipped glass that drew nothing).
//!
//! On macOS with Metal this renders for real; in GPU-less CI it skips.
use crew_render::SolidCardPass;

const SIZE: u32 = 64;
/// Row stride of the readback buffer (COPY_BYTES_PER_ROW_ALIGNMENT).
const STRIDE: usize = 256;

/// The card under test, leaving sheer page on every side of it.
const CARD: [f32; 4] = [16.0, 16.0, 32.0, 32.0];
/// A second rect — the frame asks for the focused card AND crew's chrome, so
/// one rect passing proves only half of it.
const BAR: [f32; 4] = [0.0, 56.0, 64.0, 8.0];

/// A sheer window: the page is cleared at the user's opacity, in a colour
/// that is neither black nor white so "the colour survived" is a real claim.
const CLEAR_RGB: [f64; 3] = [0.2, 0.4, 0.6];
const OPACITY: f64 = 0.6;

fn px(buf: &[u8], x: usize, y: usize) -> (u8, u8, u8, u8) {
    let o = y * STRIDE + x * 4;
    (buf[o], buf[o + 1], buf[o + 2], buf[o + 3])
}

fn render(device: &wgpu::Device, queue: &wgpu::Queue, rects: &[[f32; 4]]) -> Vec<u8> {
    let mut pass = SolidCardPass::new(device, wgpu::TextureFormat::Rgba8Unorm);
    pass.set_rects(queue, rects, (SIZE as f32, SIZE as f32));

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("solidcard_test_tex"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
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
        label: Some("solidcard_readback"),
        size: (SIZE * SIZE * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("solidcard_test"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: CLEAR_RGB[0],
                        g: CLEAR_RGB[1],
                        b: CLEAR_RGB[2],
                        a: OPACITY,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.draw(&mut rp);
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
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll failed");
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll failed");
    let data = buf.slice(..).get_mapped_range().to_vec();
    buf.unmap();
    data
}

#[test]
fn solid_card_lifts_alpha_and_touches_no_colour() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) else {
        eprintln!("solid_card_lifts_alpha_and_touches_no_colour: no GPU adapter, skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed");

    let sheer = render(&device, &queue, &[]);
    let solid = render(&device, &queue, &[CARD, BAR]);

    // Inside the card: opaque, and the same colour as before.
    let inside = (CARD[0] + CARD[2] / 2.0) as usize;
    let (r, g, b, a) = px(&solid, inside, inside);
    let (r0, g0, b0, a0) = px(&sheer, inside, inside);
    println!("inside: sheer {r0},{g0},{b0},{a0} -> solid {r},{g},{b},{a}");
    assert_eq!(a, 255, "the focused card is still see-through");
    assert_eq!(
        (r, g, b),
        (r0, g0, b0),
        "solidifying the card changed its colour — the write mask is not holding"
    );

    // The second rect is drawn too — crew's chrome stays solid whatever has
    // focus, and an instance count stuck at one would look identical from Rust.
    let bar_y = (BAR[1] + BAR[3] / 2.0) as usize;
    let (br, bg, bb, ba) = px(&solid, 8, bar_y);
    assert_eq!(ba, 255, "the second rect was never drawn");
    assert_eq!(
        (br, bg, bb),
        (r0, g0, b0),
        "the second rect changed its colour"
    );

    // Outside them: still exactly as sheer as the user asked for.
    let (_, _, _, a_out) = px(&solid, 4, 4);
    assert_eq!(
        a_out, a0,
        "the page outside the focused card lost its transparency"
    );

    // The edge lands where the rect says: the last pixel in is solid, the
    // first pixel out is not (an off-by-one here is a bright seam on screen).
    let last_in = (CARD[0] + CARD[2] - 1.0) as usize;
    let first_out = (CARD[0] + CARD[2]) as usize;
    assert_eq!(px(&solid, last_in, inside).3, 255, "the card ends early");
    assert_eq!(
        px(&solid, first_out, inside).3,
        a0,
        "the card overhangs its rect"
    );
}
