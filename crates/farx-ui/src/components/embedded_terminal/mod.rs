//! Embedded terminal session backed by a PTY and a vt100 parser.
//!
//! Split into:
//! - [`session`]: PTY spawn lifecycle, output polling, input writing, resize,
//!   vt100 parsing glue.
//! - [`input`]: key event to PTY byte encoding.
//! - [`render`]: ratatui rendering of the vt100 screen.

mod input;
mod render;
mod session;
mod thumbnail;

pub use input::key_to_bytes;
pub use render::render_terminal;
pub use session::{OutputWaker, TerminalSession};
pub use thumbnail::render_thumbnail;
