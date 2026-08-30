//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod celldeco;
mod color;
mod contrast;
mod cursor;
mod graphics;
mod graphicscmd;
mod listener;
mod model;
mod modes;
mod osc;
mod pty;
mod schemenotify;
pub use graphicscmd::ImageCmd;
pub use model::{GridSize, HeadlessTerm, PlacedImage, RenderCell, TermModel};
pub use modes::InputModes;
pub use osc::{Progress, ShellMark};
pub use pty::PtyTerm;
pub use schemenotify::scheme_report;
