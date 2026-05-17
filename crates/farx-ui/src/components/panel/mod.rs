//! File-panel renderer: paints a single directory listing with header,
//! grid, zebra-striped rows, and footer.
//!
//! Split into submodules so each file stays under the 200-line cap:
//! * [`render`] — top-level orchestration (`render_panel`)
//! * [`row`] — per-entry line painter
//! * [`header`] — column header + grid separator
//! * [`footer`] — item count / selection summary
//! * [`helpers`] — size formatting, padding helpers
//! * [`entry_kind`] — image/archive/executable classifiers

mod entry_kind;
mod footer;
mod header;
mod helpers;
mod render;
mod row;

pub use helpers::format_size;
pub use render::render_panel;
