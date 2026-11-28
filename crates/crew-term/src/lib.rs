//! crew-term: terminal model + PTY, behind a stable TermModel interface.
mod celldeco;
mod color;
mod contrast;
mod cursor;
mod graphics;
mod graphicscmd;
mod listener;
mod model;
mod modelinverse;
mod modes;
mod osc;
mod pixelmouse;
mod pointershape;
mod pty;
mod ptyident;
mod ptylocale;
mod schemenotify;
#[cfg(test)]
mod testtheme;
mod wipescan;
pub use graphicscmd::ImageCmd;
pub use model::{
    ClipboardAsk, FindSpan, GridSize, HeadlessTerm, PlacedImage, RenderCell, TermModel, FIND_CAP,
};
pub use modes::InputModes;
pub use osc::{Progress, ShellMark};
pub use pty::PtyTerm;
pub use schemenotify::scheme_report;
pub use wipescan::ImageWipe;
