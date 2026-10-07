//! THROWAWAY preview harness — renders the backdrop over simulated idle time
//! and pipes the frames to ffmpeg. Not a test; run with
//! `SWIRL_OUT=… cargo test -p crew-render --test zz_swirl_preview -- --ignored --nocapture`.
mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use crew_render::{ModernPaper, PaperBgPass};

fn env_f(name: &str, d: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

#[test]
#[ignore]
fn swirl_preview() {
    let Some((device, queue)) = common::gpu() else {
        return;
    };
    let out = std::env::var("SWIRL_OUT").expect("SWIRL_OUT");
    let w = env_f("SWIRL_W", 1536.0) as u32;
    let h = env_f("SWIRL_H", 960.0) as u32;
    let secs = env_f("SWIRL_SECS", 48.0);
    let fps = env_f("SWIRL_FPS", 30.0);
    let t0 = env_f("SWIRL_T0", 0.0);
    let pitch = env_f("SWIRL_PITCH", 4.0);
    let radius = env_f("SWIRL_RADIUS", 0.9);
    let light = env_f("SWIRL_LIGHT", 0.0) > 0.5;
    // Nebula (a `dark` pool theme) or Blossom (light), lively hue ±38°.
    let (page, pa, pb, wash, dots) = if light {
        (
            (250u8, 247, 252),
            (147u8, 51, 234),
            (219u8, 39, 119),
            0.12,
            0.16,
        )
    } else {
        (
            (19u8, 15, 26),
            (197u8, 138, 249),
            (244u8, 143, 177),
            0.15,
            0.20,
        )
    };
    let f = |c: u8| c as f32 / 255.0;
    let page_bg = [f(page.0), f(page.1), f(page.2), 1.0];
    let pass = PaperBgPass::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut ff = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
        ])
        .arg(format!("{w}x{h}"))
        .args(["-r"])
        .arg(format!("{fps}"))
        .args([
            "-i", "-", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "16",
        ])
        .arg(&out)
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg");
    let stdin = ff.stdin.as_mut().unwrap();
    let frames = (secs * fps) as u32;
    // Distraction meter: how much the page changes per second (mean and p99
    // of |Δluma| between frames × fps, in 8-bit levels), and how much of it
    // sits in the middle third vs the outer ring.
    let mut prev: Option<Vec<f32>> = None;
    let (mut sum, mut n, mut mid, mut nmid, mut rim, mut nrim) =
        (0f64, 0u64, 0f64, 0u64, 0f64, 0u64);
    let mut deltas: Vec<f32> = Vec::new();
    let orbit_ms = env_f("SWIRL_ORBIT_MS", 24_000.0);
    let hue_ms = orbit_ms * 2.0;
    for i in 0..frames {
        let t_ms = (t0 + i as f32 / fps) * 1000.0;
        let phase = (t_ms / orbit_ms).fract();
        let hue = (t_ms / hue_ms).fract();
        let deg = 38.0 * (std::f32::consts::TAU * hue).sin();
        let sa = crew_theme::poleshift::shifted(pa, deg);
        let sb = crew_theme::poleshift::shifted(pb, deg);
        let m = ModernPaper {
            color_a: [f(sa.0), f(sa.1), f(sa.2)],
            color_b: [f(sb.0), f(sb.1), f(sb.2)],
            dots,
            spacing: [pitch, pitch],
            radius,
            wash,
            clocks: crew_render::WashClocks {
                phase,
                wander: hue,
                live: (t_ms / 3000.0).min(1.0) * env_f("SWIRL_LIVE", 1.0),
                eddy: (t_ms / (orbit_ms * 2.618034)).fract(),
            },
            focus: [0.5, 0.5],
            focus_pull: 0.0,
        };
        pass.update_uniform(&queue, page_bg, (w as f32, h as f32), 1.0, 0.0, Some(&m));
        let buf = common::render_offscreen(&device, &queue, &pass, w, h);
        stdin.write_all(&buf).unwrap();
        let luma: Vec<f32> = buf
            .chunks(4)
            .map(|p| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32)
            .collect();
        if let Some(pv) = &prev {
            if i as f32 / fps > 3.5 {
                for (k, (a, b)) in luma.iter().zip(pv).enumerate().step_by(7) {
                    let d = (a - b).abs() * fps;
                    sum += d as f64;
                    n += 1;
                    deltas.push(d);
                    let (x, y) = (
                        (k as u32 % w) as f32 / w as f32 - 0.5,
                        (k as u32 / w) as f32 / h as f32 - 0.5,
                    );
                    if x.abs() < 0.17 && y.abs() < 0.17 {
                        mid += d as f64;
                        nmid += 1;
                    } else if x.abs() > 0.33 || y.abs() > 0.33 {
                        rim += d as f64;
                        nrim += 1;
                    }
                }
            }
        }
        prev = Some(luma);
    }
    drop(ff.stdin.take());
    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p99 = deltas.get(deltas.len() * 99 / 100).copied().unwrap_or(0.0);
    eprintln!(
        "METER mean |dL|/s {:.2}  p99 {:.2}  centre/rim {:.2}",
        sum / n.max(1) as f64,
        p99,
        (mid / nmid.max(1) as f64) / (rim / nrim.max(1) as f64).max(1e-6)
    );
    assert!(ff.wait().unwrap().success());
}
