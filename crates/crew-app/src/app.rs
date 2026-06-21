use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crew_render::{Gpu, TextLayer};
use crew_term::{GridSize, PtyTerm, TermModel};

use crate::session::cells_to_string;

const PTY_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
const POLL_MS: u64 = 16;

#[derive(Default)]
pub struct CrewApp {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    text: Option<TextLayer>,
    pty: Option<PtyTerm>,
}

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Crew");
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        match Gpu::new(window.clone()) {
            Ok(gpu) => {
                let text = TextLayer::new(&gpu);
                let pty = PtyTerm::spawn(PTY_SIZE, "bash")
                    .or_else(|_| PtyTerm::spawn(PTY_SIZE, "sh"))
                    .ok();
                self.gpu = Some(gpu);
                self.text = Some(text);
                self.pty = pty;
                self.window = Some(window.clone());
                window.request_redraw();
            }
            Err(e) => {
                eprintln!("GPU init failed: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(pty), Some(text), Some(window)) = (&mut self.pty, &mut self.text, &self.window)
        else {
            return;
        };

        let new_bytes = pty.try_read();
        if new_bytes > 0 {
            let s = cells_to_string(&pty.cells(), PTY_SIZE);
            text.set_text(&s);
            window.request_redraw();
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(POLL_MS),
        ));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let (Some(gpu), Some(text)) = (&self.gpu, &mut self.text) else {
                    return;
                };

                text.prepare(gpu);

                let frame = match gpu.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) => t,
                    wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => return,
                    wgpu::CurrentSurfaceTexture::Outdated
                    | wgpu::CurrentSurfaceTexture::Lost
                    | wgpu::CurrentSurfaceTexture::Validation => {
                        eprintln!("surface lost/outdated/validation — skipping frame");
                        return;
                    }
                };

                let view = frame.texture.create_view(&Default::default());
                let mut enc = gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                {
                    let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("crew frame"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.05,
                                    g: 0.05,
                                    b: 0.07,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    text.draw(&mut pass);
                }

                gpu.queue.submit(Some(enc.finish()));
                frame.present();
            }
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = CrewApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
