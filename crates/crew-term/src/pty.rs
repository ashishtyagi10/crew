use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::sync::mpsc::{channel, Receiver};

use crate::model::{GridSize, RenderCell, TermCore, TermModel};

pub struct PtyTerm {
    core: TermCore,
    master: Box<dyn portable_pty::MasterPty + Send>,
    rx: Receiver<Vec<u8>>,
    exited: bool,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyTerm {
    /// Spawn a shell (no extra args).  Delegates to `spawn_args`.
    pub fn spawn(size: GridSize, shell: &str) -> anyhow::Result<Self> {
        Self::spawn_args(size, shell, &[])
    }

    /// Spawn `command` with `args` in a new PTY of the given size.
    pub fn spawn_args(size: GridSize, command: &str, args: &[String]) -> anyhow::Result<Self> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        // Advertise a capable terminal so TUI programs behave (env is otherwise
        // inherited from the host process, so $HOME/$PATH etc. are present).
        cmd.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(cmd)?;
        // Drop the slave end so EOF propagates when the child exits.
        drop(pair.slave);

        // Spawn a reader thread: portable-pty reads are blocking.
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
            exited: false,
            _child: child,
        })
    }

    /// True once the child process has exited and all its output has been drained
    /// (the reader thread ended and the channel disconnected). Set by `try_read`.
    pub fn exited(&self) -> bool {
        self.exited
    }

    /// Returns a fresh writer to the master PTY end (sends input to the shell).
    pub fn writer(&self) -> Box<dyn std::io::Write + Send> {
        self.master.take_writer().expect("pty writer already taken")
    }

    /// Drains all pending bytes from the reader thread into the terminal model.
    /// Returns the total number of bytes consumed.
    pub fn try_read(&mut self) -> usize {
        use std::sync::mpsc::TryRecvError;
        let mut total = 0;
        loop {
            match self.rx.try_recv() {
                Ok(chunk) => {
                    total += chunk.len();
                    self.core.feed(&chunk);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Reader thread ended → child exited and output is drained.
                    self.exited = true;
                    break;
                }
            }
        }
        total
    }
}

impl PtyTerm {
    /// Scroll the viewport by `delta` lines into scrollback (positive = older).
    pub fn scroll(&mut self, delta: i32) {
        self.core.scroll(delta);
    }

    /// Jump back to the live bottom of the terminal.
    pub fn scroll_to_bottom(&mut self) {
        self.core.scroll_to_bottom();
    }

    /// Lines currently scrolled back from the live bottom (0 = at the bottom).
    pub fn display_offset(&self) -> usize {
        self.core.display_offset()
    }

    /// Whether the program enabled bracketed-paste mode.
    pub fn bracketed_paste(&self) -> bool {
        self.core.bracketed_paste()
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
