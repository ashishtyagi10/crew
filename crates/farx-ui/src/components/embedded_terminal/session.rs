use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::Arc;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

/// Callback invoked by the PTY reader thread whenever new output arrives, so
/// the render loop can be woken for an immediate redraw instead of waiting
/// for the next periodic tick. Kept as an opaque closure to avoid coupling
/// the session to the app's event types.
pub type OutputWaker = Arc<dyn Fn() + Send + Sync>;

/// An embedded terminal session backed by a PTY + vt100 parser.
pub struct TerminalSession {
    /// Stable monotonic id assigned at spawn; never changes even when other
    /// terminals are closed.
    pub id: usize,
    /// Human-readable title (e.g. "claude", "bash").
    pub title: String,
    /// vt100 terminal emulator / parser.
    parser: vt100::Parser,
    /// Channel receiving raw bytes from the PTY reader thread.
    output_rx: mpsc::Receiver<Vec<u8>>,
    /// Writer handle to send input to the PTY.
    writer: Box<dyn Write + Send>,
    /// PTY master handle (kept alive to prevent premature close).
    _master: Box<dyn portable_pty::MasterPty + Send>,
    /// Whether the child process is still running.
    pub alive: bool,
    /// Whether this terminal has unread output (for attention indicator).
    pub has_attention: bool,
    /// Current terminal dimensions.
    pub rows: u16,
    pub cols: u16,
}

impl TerminalSession {
    /// Spawn a command in a new PTY with the given dimensions and working directory.
    pub fn spawn(
        id: usize,
        cmd: &str,
        args: &[&str],
        cwd: &std::path::Path,
        rows: u16,
        cols: u16,
        waker: Option<OutputWaker>,
    ) -> anyhow::Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut command = CommandBuilder::new(cmd);
        for arg in args {
            command.arg(arg);
        }
        command.cwd(cwd);

        // Spawn the child process in the PTY slave
        let _child = pair.slave.spawn_command(command)?;
        // Drop the slave side — we only need the master
        drop(pair.slave);

        let writer = pair.master.take_writer()?;

        // Background thread to read PTY output
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let mut reader = pair.master.try_clone_reader()?;

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                        // Wake the render loop so this output is shown right
                        // away rather than on the next tick.
                        if let Some(ref wake) = waker {
                            wake();
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let parser = vt100::Parser::new(rows, cols, 100);

        Ok(Self {
            id,
            title: cmd.to_string(),
            parser,
            output_rx: rx,
            writer,
            _master: pair.master,
            alive: true,
            has_attention: false,
            rows,
            cols,
        })
    }

    /// Drain all pending PTY output into the vt100 parser.
    /// Returns true if any new output was received.
    pub fn poll_output(&mut self) -> bool {
        let mut got_data = false;
        while let Ok(data) = self.output_rx.try_recv() {
            self.parser.process(&data);
            got_data = true;
        }
        // Check if process exited (channel closed + no more data)
        if !got_data {
            if let Err(mpsc::TryRecvError::Disconnected) = self.output_rx.try_recv() {
                self.alive = false;
            }
        }
        got_data
    }

    /// Write raw bytes to the PTY (keyboard input).
    pub fn write_input(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    /// Resize the terminal.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        if rows == self.rows && cols == self.cols {
            return;
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.set_size(rows, cols);
        let _ = self._master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    /// Get the vt100 screen for rendering.
    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
}
