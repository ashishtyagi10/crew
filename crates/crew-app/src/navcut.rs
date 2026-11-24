//! Which texts the nav's last panel draw had to cut — what the hover card (`navtip`) asks
//! before it offers a row's whole text.
//!
//! A card that repeated a row the nav already showed whole would be noise, so the card needs
//! to know whether the row was cut. Re-deriving that from widths beside each section's draw
//! would be a second copy of every row's layout, free to drift from the one on screen. So the
//! one clipping call every nav row goes through ([`crate::navtext::put_at`], and the PANES
//! title's [`clip`]) notes what it shortened, while [`record`] is listening: the frame's own
//! draw is the answer.
use std::cell::RefCell;

thread_local! {
    /// The texts cut since [`record`] began, or `None` when nothing is listening.
    static CUTS: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Run `draw`, and return what it drew with every text it had to cut on the way.
pub(crate) fn record<T>(draw: impl FnOnce() -> T) -> (T, Vec<String>) {
    let outer = CUTS.with(|c| c.replace(Some(Vec::new())));
    let out = draw();
    let cut = CUTS.with(|c| c.replace(outer)).unwrap_or_default();
    (out, cut)
}

/// Note that `s` was not drawn whole — cut, or left out for want of room.
pub(crate) fn note(s: &str) {
    CUTS.with(|c| {
        if let Some(v) = c.borrow_mut().as_mut() {
            v.push(s.to_string());
        }
    });
}

/// [`crate::chatwidth::clip_words`], noting `s` when it comes back shorter.
pub(crate) fn clip(s: &str, max: usize) -> String {
    let fit = crate::chatwidth::clip_words(s, max);
    if fit != s {
        note(s);
    }
    fit
}
