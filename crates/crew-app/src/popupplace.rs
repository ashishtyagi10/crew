//! Where a composer pop-up sits: the "commands" palette, the attach picker,
//! the model picker, the key prompt, Cmd+F and Ctrl+R all stand ABOVE the
//! focused crew pane's composer. The composer is not the pane's last rows:
//! `chatplace::grants` puts the summary footer (model · cost · mode line,
//! up to three rows) under it. Every pop-up used to subtract the composer
//! alone, so it landed on the composer itself and hid the very text being
//! chosen — the query you typed, the row you were about to accept. One
//! function, one source of the composer's top: the same grants the pane
//! is drawn from.
use crate::chat::ChatPane;
use crate::layout::Rect;

/// The y (surface px) of a pop-up `mh` px tall whose bottom edge meets the
/// composer's top edge in pane `r`; clamped at the pane's top when the
/// pop-up is taller than the room above.
pub(crate) fn above_composer(pane: &ChatPane, r: Rect, cw: f32, ch: f32, mh: f32) -> f32 {
    let cols = (r.w / cw).floor() as u16;
    let rows = (r.h / ch).floor() as u16;
    let g = crate::chatplace::grants(pane, cols, rows);
    let composer_top = r.y + f32::from(rows.saturating_sub(g.bottom)) * ch;
    (composer_top - mh).max(r.y)
}

#[cfg(test)]
#[path = "popupplace_tests.rs"]
mod tests;
