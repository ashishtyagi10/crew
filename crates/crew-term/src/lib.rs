//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod color;
mod cursor;
mod model;
mod pty;
pub use model::{GridSize, HeadlessTerm, RenderCell, TermModel};
pub use pty::PtyTerm;
