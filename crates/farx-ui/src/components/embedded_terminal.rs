use std::io::{Read, Write};
use std::sync::mpsc;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// An embedded terminal session backed by a PTY + vt100 parser.
pub struct TerminalSession {
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
        cmd: &str,
        args: &[&str],
        cwd: &std::path::Path,
        rows: u16,
        cols: u16,
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
                    }
                    Err(_) => break,
                }
            }
        });

        let parser = vt100::Parser::new(rows, cols, 100);

        Ok(Self {
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

/// Convert a crossterm KeyEvent into raw bytes to send to the PTY.
pub fn key_to_bytes(key: &crossterm::event::KeyEvent) -> Option<Vec<u8>> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Char(c) if ctrl => {
            // Ctrl+A = 0x01, Ctrl+B = 0x02, ..., Ctrl+Z = 0x1A
            let byte = (c.to_ascii_lowercase() as u8)
                .wrapping_sub(b'a')
                .wrapping_add(1);
            if byte <= 26 {
                Some(vec![byte])
            } else {
                None
            }
        }
        KeyCode::Char(c) if alt => {
            let mut bytes = vec![0x1b]; // ESC prefix for Alt
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            Some(bytes)
        }
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            Some(c.encode_utf8(&mut buf).as_bytes().to_vec())
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::BackTab => Some(vec![0x1b, b'[', b'Z']),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(vec![0x1b, b'[', b'A']),
        KeyCode::Down => Some(vec![0x1b, b'[', b'B']),
        KeyCode::Right => Some(vec![0x1b, b'[', b'C']),
        KeyCode::Left => Some(vec![0x1b, b'[', b'D']),
        KeyCode::Home => Some(vec![0x1b, b'[', b'H']),
        KeyCode::End => Some(vec![0x1b, b'[', b'F']),
        KeyCode::PageUp => Some(vec![0x1b, b'[', b'5', b'~']),
        KeyCode::PageDown => Some(vec![0x1b, b'[', b'6', b'~']),
        KeyCode::Insert => Some(vec![0x1b, b'[', b'2', b'~']),
        KeyCode::Delete => Some(vec![0x1b, b'[', b'3', b'~']),
        KeyCode::F(n) => {
            let seq = match n {
                1 => "\x1bOP",
                2 => "\x1bOQ",
                3 => "\x1bOR",
                4 => "\x1bOS",
                5 => "\x1b[15~",
                6 => "\x1b[17~",
                7 => "\x1b[18~",
                8 => "\x1b[19~",
                9 => "\x1b[20~",
                10 => "\x1b[21~",
                11 => "\x1b[23~",
                12 => "\x1b[24~",
                _ => return None,
            };
            Some(seq.as_bytes().to_vec())
        }
        _ => None,
    }
}

/// Convert a vt100 color to a ratatui color.
fn vt100_to_ratatui_color(color: vt100::Color, default: Color) -> Color {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Idx(n) => Color::Indexed(n),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Render a terminal session into a frame area.
pub fn render_terminal(frame: &mut Frame, area: Rect, session: &TerminalSession, is_focused: bool) {
    let border_color = if is_focused {
        Color::Cyan
    } else if session.has_attention {
        Color::Yellow
    } else if !session.alive {
        Color::Red
    } else {
        Color::Indexed(240)
    };

    let title = if !session.alive {
        format!(" {} (exited) ", session.title)
    } else {
        format!(" {} ", session.title)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color).bg(Color::Black))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let screen = session.screen();
    let screen_rows = inner.height as usize;
    let screen_cols = inner.width as usize;

    let mut lines: Vec<Line<'_>> = Vec::with_capacity(screen_rows);

    for row in 0..screen_rows {
        let mut spans: Vec<Span<'_>> = Vec::new();
        let mut col = 0usize;

        while col < screen_cols {
            let cell = screen.cell(row as u16, col as u16);
            match cell {
                Some(cell) => {
                    let fg = vt100_to_ratatui_color(cell.fgcolor(), Color::White);
                    let bg = vt100_to_ratatui_color(cell.bgcolor(), Color::Black);
                    let mut style = Style::default().fg(fg).bg(bg);
                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    if cell.inverse() {
                        style = style.add_modifier(Modifier::REVERSED);
                    }

                    let contents = cell.contents();
                    if contents.is_empty() {
                        spans.push(Span::styled(" ", style));
                    } else {
                        spans.push(Span::styled(contents.to_string(), style));
                    }
                    col += 1;
                }
                None => {
                    spans.push(Span::styled(
                        " ",
                        Style::default().fg(Color::White).bg(Color::Black),
                    ));
                    col += 1;
                }
            }
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // Show cursor if terminal is focused and cursor is visible
    if is_focused {
        let cursor = screen.cursor_position();
        let cx = inner.x + cursor.1;
        let cy = inner.y + cursor.0;
        if cx < inner.x + inner.width && cy < inner.y + inner.height {
            frame.set_cursor_position((cx, cy));
        }
    }
}
