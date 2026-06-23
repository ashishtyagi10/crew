# Crew Foundation (Single-Pane GPU Terminal) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native, GPU-rendered terminal window in Rust that runs one real interactive shell — the walking skeleton that proves the winit → wgpu → glyphon → alacritty_terminal → portable-pty stack.

**Architecture:** A Cargo workspace. `crew-term` wraps `alacritty_terminal::Term` (grid/scrollback/damage) fed by a `portable-pty` child, behind a stable internal `TermModel` interface. `crew-render` owns the winit window, the wgpu surface, and glyphon text drawing. `crew-app` is the binary that wires them: keystrokes → PTY, PTY output → `Term` → rendered each frame.

**Tech Stack:** Rust (stable), `winit` 0.30, `wgpu` 29, `glyphon` 0.11, `cosmic-text` 0.19 (via glyphon), `alacritty_terminal` 0.26, `portable-pty` 0.9.

## Global Constraints

- **Rust edition 2021**, stable toolchain.
- **Pin exact dependency versions** (`=x.y.z`) — `alacritty_terminal` and `wgpu` ship breaking changes in normal releases. Verify every API used below against `docs.rs` for the *pinned* version before implementing; adjust signatures to match the pinned release (these crates move; the compile loop is the source of truth).
- **All deps must be MIT/Apache/Zlib.** No AGPL.
- **File-size discipline:** keep `.rs` files focused and small (Crew inherits the prior project's small-file habit); split a file by responsibility before it grows unwieldy.
- **Isolate upstream churn behind adapters:** the renderer never imports `alacritty_terminal` types directly — only `crew-term`'s `TermModel`.
- **`cargo fmt` + `cargo clippy --all-targets` stay clean** (warning-free) at every commit.

---

### Task 1: Workspace scaffold + pinned dependencies

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/crew-term/Cargo.toml`, `crates/crew-term/src/lib.rs`
- Create: `crates/crew-render/Cargo.toml`, `crates/crew-render/src/lib.rs`
- Create: `crates/crew-app/Cargo.toml`, `crates/crew-app/src/main.rs`

**Interfaces:**
- Produces: a buildable 3-crate workspace (`crew-term`, `crew-render` libs; `crew-app` bin).

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/crew-term", "crates/crew-render", "crates/crew-app"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
winit = "=0.30.13"
wgpu = "=29.0.3"
glyphon = "=0.11.0"
alacritty_terminal = "=0.26.0"
portable-pty = "=0.9.0"
anyhow = "1"
```

- [ ] **Step 2: Create `crates/crew-term/Cargo.toml`**

```toml
[package]
name = "crew-term"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
alacritty_terminal.workspace = true
portable-pty.workspace = true
anyhow.workspace = true
```

- [ ] **Step 3: Create `crates/crew-render/Cargo.toml`**

```toml
[package]
name = "crew-render"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
winit.workspace = true
wgpu.workspace = true
glyphon.workspace = true
anyhow.workspace = true
```

- [ ] **Step 4: Create `crates/crew-app/Cargo.toml`**

```toml
[package]
name = "crew-app"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "crew"
path = "src/main.rs"

[dependencies]
crew-term = { path = "../crew-term" }
crew-render = { path = "../crew-render" }
winit.workspace = true
anyhow.workspace = true
```

- [ ] **Step 5: Create placeholder lib/main files so the workspace builds**

`crates/crew-term/src/lib.rs`:
```rust
//! crew-term: terminal model + PTY, behind a stable TermModel interface.
```

`crates/crew-render/src/lib.rs`:
```rust
//! crew-render: winit window + wgpu surface + glyphon text.
```

`crates/crew-app/src/main.rs`:
```rust
fn main() {
    println!("crew: starting");
}
```

- [ ] **Step 6: Verify it builds**

Run: `cargo build`
Expected: compiles, all three crates. (If a pinned version fails to resolve, run `cargo update -p <crate> --precise <ver>` and reconcile, keeping versions pinned.)

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/
git commit -m "chore: scaffold crew workspace (crew-term, crew-render, crew-app)"
```

---

### Task 2: `crew-term` — Term + PTY behind `TermModel` (TDD)

**Files:**
- Modify: `crates/crew-term/src/lib.rs`
- Create: `crates/crew-term/src/model.rs`

**Interfaces:**
- Produces:
  - `struct GridSize { pub cols: u16, pub rows: u16 }`
  - `struct RenderCell { pub col: u16, pub row: u16, pub c: char }` (color comes in Task 6's render mapping; v1 of the adapter exposes char + position to keep the first test small)
  - `trait TermModel`:
    - `fn feed(&mut self, bytes: &[u8])`
    - `fn cells(&self) -> Vec<RenderCell>` (every non-empty cell in the visible grid)
    - `fn resize(&mut self, size: GridSize)`
  - `struct PtyTerm` implementing `TermModel`, plus:
    - `fn spawn(size: GridSize, shell: &str) -> anyhow::Result<PtyTerm>`
    - `fn writer(&self) -> Box<dyn std::io::Write + Send>` (for keystrokes)
    - `fn try_read(&mut self) -> usize` (drain pending PTY bytes into the Term; returns bytes consumed)

- [ ] **Step 1: Write the failing test** (`crates/crew-term/src/model.rs`, `#[cfg(test)]` at bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // A headless Term (no PTY) we can feed bytes into deterministically.
    #[test]
    fn feeding_text_appears_in_cells() {
        let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 5 });
        term.feed(b"hi");
        let cells = term.cells();
        let text: String = {
            let mut row0: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
            row0.sort_by_key(|c| c.col);
            row0.iter().map(|c| c.c).collect()
        };
        assert_eq!(text, "hi");
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-term`
Expected: FAIL — `HeadlessTerm`, `GridSize`, `feed`, `cells` not found.

- [ ] **Step 3: Implement `GridSize`, `RenderCell`, `TermModel`, and a `HeadlessTerm`**

`crates/crew-term/src/model.rs` (top):
```rust
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

#[derive(Clone, Copy, Debug)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct RenderCell {
    pub col: u16,
    pub row: u16,
    pub c: char,
}

pub trait TermModel {
    fn feed(&mut self, bytes: &[u8]);
    fn cells(&self) -> Vec<RenderCell>;
    fn resize(&mut self, size: GridSize);
}

// alacritty_terminal needs a Dimensions impl describing the viewport.
#[derive(Clone, Copy)]
struct Dims {
    cols: usize,
    rows: usize,
}
impl Dimensions for Dims {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

// A no-op event listener — we don't react to terminal events yet.
#[derive(Clone)]
struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _event: Event) {}
}

// Shared core: a Term + an ANSI processor. Used by HeadlessTerm and PtyTerm.
struct TermCore {
    term: Term<NoopListener>,
    parser: Processor,
}

impl TermCore {
    fn new(size: GridSize) -> Self {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        let term = Term::new(Config::default(), &dims, NoopListener);
        Self {
            term,
            parser: Processor::new(),
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    fn cells(&self) -> Vec<RenderCell> {
        let content = self.term.renderable_content();
        content
            .display_iter
            .filter(|ind| ind.c != ' ' && ind.c != '\0')
            .map(|ind| RenderCell {
                col: ind.point.column.0 as u16,
                row: ind.point.line.0 as u16,
                c: ind.c,
            })
            .collect()
    }

    fn resize(&mut self, size: GridSize) {
        let dims = Dims {
            cols: size.cols as usize,
            rows: size.rows as usize,
        };
        self.term.resize(dims);
    }
}

pub struct HeadlessTerm {
    core: TermCore,
}

impl HeadlessTerm {
    pub fn new(size: GridSize) -> Self {
        Self {
            core: TermCore::new(size),
        }
    }
}

impl TermModel for HeadlessTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }
    fn cells(&self) -> Vec<RenderCell> {
        self.core.cells()
    }
    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
    }
}
```

> Verify against docs.rs for `alacritty_terminal` 0.26.0: the exact paths of `Processor`/`advance`, `Term::new`/`resize` signatures, `Config`, the `Dimensions` trait methods, and `renderable_content().display_iter` item fields (`point.column.0`, `point.line.0`, `.c`). Adjust imports/calls to match the pinned release.

- [ ] **Step 4: Wire the module into the crate**

`crates/crew-term/src/lib.rs`:
```rust
//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod model;
pub use model::{GridSize, HeadlessTerm, RenderCell, TermModel};
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p crew-term`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/crew-term
git commit -m "feat(crew-term): Term model behind TermModel, with headless test"
```

---

### Task 3: `crew-term` — `PtyTerm` driving a real shell (TDD)

**Files:**
- Modify: `crates/crew-term/src/model.rs`, `crates/crew-term/src/lib.rs`

**Interfaces:**
- Produces: `PtyTerm` (implements `TermModel`) with `spawn`, `writer`, `try_read` as listed in Task 2's interface block.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod pty_tests {
    use super::*;
    use std::io::Write;
    use std::time::{Duration, Instant};

    #[test]
    fn echo_roundtrips_through_pty() {
        let mut term = PtyTerm::spawn(GridSize { cols: 40, rows: 10 }, "sh").unwrap();
        let mut w = term.writer();
        // Echo a unique token, then read until it shows up on the grid.
        w.write_all(b"printf CREWOK\n").unwrap();
        w.flush().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut found = false;
        while Instant::now() < deadline {
            term.try_read();
            let line: String = {
                let mut cs: Vec<_> = term.cells();
                cs.sort_by_key(|c| (c.row, c.col));
                cs.iter().map(|c| c.c).collect()
            };
            if line.contains("CREWOK") {
                found = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(found, "expected CREWOK to appear on the terminal grid");
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p crew-term pty_tests`
Expected: FAIL — `PtyTerm` not found.

- [ ] **Step 3: Implement `PtyTerm`**

Append to `crates/crew-term/src/model.rs`:
```rust
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::sync::mpsc::{channel, Receiver};

pub struct PtyTerm {
    core: TermCore,
    master: Box<dyn MasterPty + Send>,
    rx: Receiver<Vec<u8>>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyTerm {
    pub fn spawn(size: GridSize, shell: &str) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let child = pair.slave.spawn_command(CommandBuilder::new(shell))?;
        drop(pair.slave);

        // Reader thread → channel (portable-pty is blocking).
        let mut reader = pair.master.try_clone_reader()?;
        let (tx, rx) = channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            core: TermCore::new(size),
            master: pair.master,
            rx,
            _child: child,
        })
    }

    pub fn writer(&self) -> Box<dyn std::io::Write + Send> {
        self.master.take_writer().expect("pty writer")
    }

    pub fn try_read(&mut self) -> usize {
        let mut total = 0;
        while let Ok(chunk) = self.rx.try_recv() {
            total += chunk.len();
            self.core.feed(&chunk);
        }
        total
    }
}

impl TermModel for PtyTerm {
    fn feed(&mut self, bytes: &[u8]) {
        self.core.feed(bytes);
    }
    fn cells(&self) -> Vec<RenderCell> {
        self.core.cells()
    }
    fn resize(&mut self, size: GridSize) {
        self.core.resize(size);
        let _ = self.master.resize(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}
```

Update `crates/crew-term/src/lib.rs`:
```rust
//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod model;
pub use model::{GridSize, HeadlessTerm, PtyTerm, RenderCell, TermModel};
```

> Verify against docs.rs for `portable-pty` 0.9.0: `openpty`/`spawn_command` return types, `try_clone_reader`, `take_writer`, `resize`, and the `Child` trait bounds. Adjust as needed.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p crew-term pty_tests -- --nocapture`
Expected: PASS (CREWOK appears within the timeout).

- [ ] **Step 5: Commit**

```bash
git add crates/crew-term
git commit -m "feat(crew-term): PtyTerm drives a real shell via portable-pty"
```

---

### Task 4: `crew-render` — open a winit window (build-run-observe)

**Files:**
- Modify: `crates/crew-render/src/lib.rs`
- Create: `crates/crew-render/src/app.rs`
- Modify: `crates/crew-app/src/main.rs`

> Window/GPU code is not unit-testable; verification is "build, run, observe."

**Interfaces:**
- Produces: `struct CrewApp` implementing `winit::application::ApplicationHandler`, and `fn run() -> anyhow::Result<()>` that creates the event loop and runs it.

- [ ] **Step 1: Implement the winit `ApplicationHandler` skeleton**

`crates/crew-render/src/app.rs`:
```rust
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use std::sync::Arc;

#[derive(Default)]
pub struct CrewApp {
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for CrewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes().with_title("Crew");
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
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
```

`crates/crew-render/src/lib.rs`:
```rust
//! crew-render: winit window + wgpu surface + glyphon text.
mod app;
pub use app::{run, CrewApp};
```

- [ ] **Step 2: Call it from the binary**

`crates/crew-app/src/main.rs`:
```rust
fn main() -> anyhow::Result<()> {
    crew_render::run()
}
```

- [ ] **Step 3: Build, run, observe**

Run: `cargo run -p crew-app`
Expected: a blank window titled "Crew" opens; closing it exits cleanly. (On macOS this must run on the main thread — `cargo run` does this by default.)

- [ ] **Step 4: Commit**

```bash
git add crates/crew-render crates/crew-app
git commit -m "feat(crew-render): open a winit window"
```

---

### Task 5: `crew-render` — wgpu surface + clear color (build-run-observe)

**Files:**
- Modify: `crates/crew-render/src/app.rs`
- Create: `crates/crew-render/src/gpu.rs`

**Interfaces:**
- Produces: `struct Gpu { device, queue, surface, config, format }` with `Gpu::new(window: Arc<Window>) -> anyhow::Result<Gpu>`, `Gpu::resize(&mut self, w, h)`, and `Gpu::frame_clear(&self) -> anyhow::Result<()>` (acquires a frame and clears it). These are consumed by `crew-render` only.

- [ ] **Step 1: Implement the GPU bring-up**

`crates/crew-render/src/gpu.rs`:
```rust
use std::sync::Arc;
use winit::window::Window;

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
}

impl Gpu {
    pub fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        ))?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        Ok(Self { device, queue, surface, config, format })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    pub fn frame_clear(&self) -> anyhow::Result<()> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.07, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(Some(enc.finish()));
        frame.present();
        Ok(())
    }
}
```

Add to `crew-render/Cargo.toml` dependencies: `pollster = "0.4"`.

> Verify against docs.rs for `wgpu` 29.0.3: `Instance::default`/`create_surface` lifetime, `request_adapter`/`request_device` signatures (some 29.x revisions return `Result`, others `Option`/tuple — match the pinned one), and `SurfaceConfiguration` fields. Adjust `pollster`/`block_on` if the pinned wgpu exposes blocking helpers.

- [ ] **Step 2: Hold `Gpu` in the app and clear each frame**

In `crates/crew-render/src/app.rs`, add `gpu: Option<crate::gpu::Gpu>` to `CrewApp`; in `resumed`, after creating the window, set `self.gpu = Some(crate::gpu::Gpu::new(window.clone())?)` (change `resumed` to handle the Result — log and exit on error). In `window_event`, handle `WindowEvent::Resized(size)` → `gpu.resize(size.width, size.height)`, and on `RedrawRequested` call `gpu.frame_clear()` then `window.request_redraw()`.

- [ ] **Step 3: Build, run, observe**

Run: `cargo run -p crew-app`
Expected: the window is filled with a dark blue-grey clear color; resizing keeps it filled without panics.

- [ ] **Step 4: Commit**

```bash
git add crates/crew-render
git commit -m "feat(crew-render): wgpu surface with per-frame clear"
```

---

### Task 6: `crew-render` — draw monospace text with glyphon (build-run-observe)

**Files:**
- Create: `crates/crew-render/src/text.rs`
- Modify: `crates/crew-render/src/app.rs`, `crates/crew-render/src/gpu.rs`

**Interfaces:**
- Produces: `struct TextLayer` with `TextLayer::new(gpu) -> TextLayer`, `TextLayer::set_text(&mut self, &str)`, and `TextLayer::draw(&mut self, gpu, &mut pass)` — renders a string as monospace text. Consumed by `crew-render`.

- [ ] **Step 1: Implement the glyphon text layer**

`crates/crew-render/src/text.rs`:
```rust
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use crate::gpu::Gpu;

pub struct TextLayer {
    font_system: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffer: Buffer,
}

impl TextLayer {
    pub fn new(gpu: &Gpu) -> Self {
        let mut font_system = FontSystem::new();
        let swash = SwashCache::new();
        let cache = Cache::new(&gpu.device);
        let viewport = Viewport::new(&gpu.device, &cache);
        let mut atlas = TextAtlas::new(&gpu.device, &gpu.queue, &cache, gpu.format);
        let renderer =
            TextRenderer::new(&mut atlas, &gpu.device, wgpu::MultisampleState::default(), None);
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));
        buffer.set_text(&mut font_system, "crew ready", Attrs::new().family(Family::Monospace), Shaping::Advanced);
        Self { font_system, swash, viewport, atlas, renderer, buffer }
    }

    pub fn set_text(&mut self, text: &str) {
        self.buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );
    }

    pub fn prepare(&mut self, gpu: &Gpu) {
        self.viewport.update(
            &gpu.queue,
            Resolution { width: gpu.config.width, height: gpu.config.height },
        );
        let area = TextArea {
            buffer: &self.buffer,
            left: 8.0,
            top: 8.0,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: gpu.config.width as i32,
                bottom: gpu.config.height as i32,
            },
            default_color: Color::rgb(220, 220, 220),
            custom_glyphs: &[],
        };
        self.renderer
            .prepare(&gpu.device, &gpu.queue, &mut self.font_system, &mut self.atlas, &self.viewport, [area], &mut self.swash)
            .unwrap();
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        self.renderer.render(&self.atlas, &self.viewport, pass).unwrap();
    }
}
```

> Verify against docs.rs for `glyphon` 0.11.0: `Cache`/`Viewport`/`TextAtlas::new` argument order, `TextArea` fields (esp. `custom_glyphs`), and `prepare`/`render` signatures. These changed across 0.9→0.11 — match the pinned release exactly.

- [ ] **Step 2: Refactor `frame_clear` into `frame_with(text)`**

Replace `Gpu::frame_clear` usage: add a method that opens the same render pass but, before ending it, calls `text.prepare(self)` (outside the pass) and `text.draw(pass)` (inside the pass). Keep the clear `load`. The simplest shape: in `app.rs`'s redraw handler, call `text.prepare(gpu)`, then acquire the frame, begin the pass with the clear, call `text.draw(&mut pass)`, end pass, submit, present. Move the frame acquisition into `app.rs` (or pass `&TextLayer` into a `Gpu::frame(&self, text)` method).

- [ ] **Step 3: Hold a `TextLayer` in `CrewApp`, create it after `Gpu`, draw "crew ready"**

- [ ] **Step 4: Build, run, observe**

Run: `cargo run -p crew-app`
Expected: "crew ready" is drawn in light-grey monospace on the dark background.

- [ ] **Step 5: Commit**

```bash
git add crates/crew-render
git commit -m "feat(crew-render): monospace text rendering via glyphon"
```

---

### Task 7: `crew-app` — render the live terminal grid (build-run-observe)

**Files:**
- Modify: `crates/crew-app/src/main.rs`
- Create: `crates/crew-app/src/session.rs`

**Interfaces:**
- Consumes: `crew_term::{PtyTerm, GridSize, TermModel, RenderCell}`, `crew_render::CrewApp` internals (we extend `CrewApp` to own a `PtyTerm` and convert its cells to a single string the `TextLayer` draws).
- Produces: a redraw path that, each frame, calls `pty.try_read()` and rebuilds the on-screen text from `pty.cells()`.

- [ ] **Step 1: Add a cells→string helper** (`crates/crew-app/src/session.rs`)

```rust
use crew_term::{RenderCell, GridSize};

/// Flatten visible cells into rows of text for the (temporary) single-string renderer.
pub fn cells_to_string(cells: &[RenderCell], size: GridSize) -> String {
    let mut grid = vec![vec![' '; size.cols as usize]; size.rows as usize];
    for c in cells {
        if (c.row as usize) < grid.len() && (c.col as usize) < grid[0].len() {
            grid[c.row as usize][c.col as usize] = c.c;
        }
    }
    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
```

(Glyphon's `Buffer` renders multi-line text, so a `\n`-joined grid displays as rows. A true per-cell renderer comes in Plan 2; this keeps Plan 1's deliverable small.)

- [ ] **Step 2: Give `CrewApp` a `PtyTerm`**

Extend `crew-render`'s `CrewApp` (or wrap it) so it owns `Option<PtyTerm>`, spawned in `resumed` with a fixed `GridSize { cols: 80, rows: 24 }` and shell `"bash"` (fallback `"sh"`). On `RedrawRequested`: `pty.try_read()`, build the string via `cells_to_string`, `text.set_text(&s)`, then prepare+draw as in Task 6.

> To keep crate boundaries clean, expose a small hook on `CrewApp` (e.g. a `frame_text(&mut self) -> String` closure or a trait) rather than importing `crew-term` into `crew-render`. Simplest for Plan 1: move the `ApplicationHandler` impl into `crew-app` and keep `crew-render` providing `Gpu` + `TextLayer` only. Prefer this — it preserves the "renderer doesn't know about terminals" boundary from the spec.

- [ ] **Step 3: Build, run, observe**

Run: `cargo run -p crew-app`
Expected: a real `bash` prompt renders in the window; it updates as the shell emits output (e.g. you see the prompt string).

- [ ] **Step 4: Commit**

```bash
git add crates/crew-app crates/crew-render
git commit -m "feat(crew-app): render the live PTY terminal grid"
```

---

### Task 8: `crew-app` — keyboard input → PTY (build-run-observe)

**Files:**
- Modify: `crates/crew-app/src/main.rs` (or wherever the `ApplicationHandler` now lives), `crates/crew-app/src/session.rs`

**Interfaces:**
- Produces: `fn key_to_bytes(event: &winit::event::KeyEvent) -> Option<Vec<u8>>` mapping a key press to PTY bytes; wired so typed keys reach the shell.

- [ ] **Step 1: Implement key→bytes** (`session.rs`)

```rust
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

pub fn key_to_bytes(event: &KeyEvent) -> Option<Vec<u8>> {
    if !event.state.is_pressed() {
        return None;
    }
    match &event.logical_key {
        Key::Named(NamedKey::Enter) => Some(b"\r".to_vec()),
        Key::Named(NamedKey::Backspace) => Some(vec![0x7f]),
        Key::Named(NamedKey::Tab) => Some(b"\t".to_vec()),
        Key::Named(NamedKey::Escape) => Some(vec![0x1b]),
        Key::Named(NamedKey::Space) => Some(b" ".to_vec()),
        Key::Character(s) => Some(s.as_bytes().to_vec()),
        _ => None,
    }
}
```

> Verify against docs.rs for `winit` 0.30: `KeyEvent.state.is_pressed()`, `logical_key`, `Key::Character`/`Key::Named`, `NamedKey` variants.

- [ ] **Step 2: Keep a PTY writer and feed it on key events**

In `resumed`, after spawning `PtyTerm`, store `self.writer = Some(pty.writer())`. In `window_event`, handle `WindowEvent::KeyboardInput { event, .. }`: if `key_to_bytes(&event)` is `Some(bytes)`, `writer.write_all(&bytes); writer.flush();`. Request a redraw afterward.

- [ ] **Step 3: Build, run, observe**

Run: `cargo run -p crew-app`
Expected: you can type `ls<Enter>` and see the shell respond inside the window. Backspace and Enter work.

- [ ] **Step 4: Commit**

```bash
git add crates/crew-app
git commit -m "feat(crew-app): route keyboard input to the PTY shell"
```

---

### Task 9: Cleanup pass + milestone verification

**Files:** all of the above.

- [ ] **Step 1: Format and lint**

Run: `cargo fmt --all && cargo clippy --all-targets`
Expected: no warnings. Fix any (remove dead code, don't `#[allow]` it).

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`
Expected: `crew-term` tests pass (`feeding_text_appears_in_cells`, `echo_roundtrips_through_pty`).

- [ ] **Step 3: Manual smoke checklist**

Verify, and note results in the commit body:
- Window opens titled "Crew".
- A `bash`/`sh` prompt renders.
- Typing runs commands; output appears.
- Resizing the window doesn't panic.
- Closing the window exits cleanly.

- [ ] **Step 4: Commit the milestone**

```bash
git add -A
git commit -m "chore: Crew foundation milestone — single-pane GPU terminal works"
```

---

## Notes for the next plan (Plan 2: Grid + focus)

- Replace the `cells_to_string` shim with a **per-cell renderer** (colors, bold/italic from `RenderCell` extended with fg/bg/flags) — Task 2's `RenderCell` is the seam to extend.
- Introduce multiple `PtyTerm`s and the grid geometry (`cols = ceil(sqrt(n))`, LRU demotion) ported from the prior project.
- Add the focus model (single focus owner, click-to-focus, one reserved jump chord, passthrough).
- Add a damage-driven redraw (use `Term::damage()` via a `TermModel::take_damage()` addition) instead of rebuilding the whole string each frame.
