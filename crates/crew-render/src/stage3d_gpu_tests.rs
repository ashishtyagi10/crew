//! The 3D backdrop on a real device: threers draws on crew's device into
//! crew's format, and the pass lays it down where the scene says. Skips on a
//! machine with no GPU adapter, like the headless tests.
use super::*;

const W: u32 = 480;
const H: u32 = 300;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::None,
        ..Default::default()
    }))
    .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

/// One frame of the pass alone, over a clear page, as RGBA bytes.
fn shot(stage: &mut Stage3dPass, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
    stage.prepare(device, queue, (W, H));
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    let mut enc = device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
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
        stage.draw(&mut pass);
    }
    let row = (W * 4).div_ceil(256) * 256;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        tex.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let data = buf.slice(..).get_mapped_range().expect("mapped").to_vec();
    (0..H as usize)
        .flat_map(|y| data[y * row as usize..][..(W * 4) as usize].to_vec())
        .collect()
}

/// Covered pixels (alpha above a trace) in the box `x0..x1, y0..y1`.
fn covered(px: &[u8], (x0, x1): (u32, u32), (y0, y1): (u32, u32)) -> usize {
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| px[((y * W + x) * 4 + 3) as usize] > 8)
        .count()
}

#[test]
fn the_orbit_draws_low_on_the_right_and_off_draws_nothing() {
    let _g = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter, skipping");
        return;
    };
    let mut stage = Stage3dPass::new(&device, FORMAT);
    set_stage(Some(Stage::Orbit));
    set_phase(0.1);
    let px = shot(&mut stage, &device, &queue);
    set_stage(None);
    if let Ok(dir) = std::env::var("CREW_SHOT_DIR") {
        let path = format!("{dir}/stage3d-orbit.png");
        image::save_buffer(&path, &px, W, H, image::ColorType::Rgba8).unwrap();
        eprintln!("wrote {path}");
    }
    let right_low = covered(&px, (W / 2, W), (H / 2, H));
    let left_high = covered(&px, (0, W / 3), (0, H / 3));
    assert!(right_low > 2_000, "the knot and its ring: {right_low} px");
    assert!(
        left_high < right_low / 10,
        "the text's corner stays clear: {left_high} px"
    );
    let brightest = px.chunks(4).map(|p| p[3]).max().unwrap_or(0);
    assert!(brightest < 250, "dimmed under the panes: alpha {brightest}");

    let off = shot(&mut stage, &device, &queue);
    assert!(off.chunks(4).all(|p| p[3] == 0), "off draws nothing");
    assert!(stage.live.is_none(), "and holds nothing");
}
