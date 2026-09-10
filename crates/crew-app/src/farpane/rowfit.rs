//! One listing row's name and size fitted to the panel: the size stays
//! whole and the name gives way with a mark — for a directory as much as a
//! file, and measured in display columns, since a filename can be anything.
//!
//! Child module of [`super`] (shares its private helpers) so `render.rs`
//! stays under its line cap.

/// `(name, pad)`: the name as it will be drawn and the blank columns that
/// push `size` to the right edge of `width`.
pub(super) fn fit(name: String, size: &str, width: usize) -> (String, usize) {
    use crate::chatwidth::{clip_w, str_w};
    let name = if str_w(&name) + str_w(size) >= width {
        // A file keeps ` size`; a directory has no size and just marks its end.
        let air = if size.is_empty() { 0 } else { 1 };
        clip_w(&name, width.saturating_sub(str_w(size) + air))
    } else {
        name
    };
    let pad = width.saturating_sub(str_w(&name) + str_w(size));
    (name, pad)
}

#[cfg(test)]
#[path = "rowfit_tests.rs"]
mod tests;
