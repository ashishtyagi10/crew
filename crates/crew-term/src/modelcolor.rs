//! Answering colour queries (OSC 4/10/11/12) with the colour the pane draws.
//!
//! alacritty raises the query while parsing, to a listener that can't see the
//! palette, so it used to be answered from the theme alone. A program that
//! set a colour and read it back (vim's background probe, base16-shell, tmux)
//! was told the theme's instead. The palette is `term.colors()`: OSC 4/10/11
//! write it, OSC 104/110/111 clear it, and a theme switch never touches it —
//! so a program's colour outlives a theme change and a reset falls back to
//! the theme. Child module of `model`, split for the line cap.
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{NamedColor, Rgb};

use super::TermCore;
use crate::color::{default_bg, query_color};

/// The colour slot `index` is drawn in, which is what a query is told.
///
/// - OSC 4 slots and the foreground draw as set, so they answer as set.
/// - The background is set on every blank cell, and those still pass the
///   echo-grey drop (`should_drop_bg`): a grey or (on a dark theme) bright
///   background is flattened to the theme's. Saying "white" while the pane
///   stays dark would have vim pick its light scheme on a dark page, so the
///   answer is the colour that survives the drop.
/// - The cursor is drawn in the theme's colour whatever OSC 12 asked, so it
///   keeps the theme's answer.
pub(super) fn drawn(index: usize, palette: &Colors) -> Option<Rgb> {
    let set = match palette[index] {
        Some(c) if index != NamedColor::Cursor as usize => c,
        _ => return query_color(index),
    };
    if index == NamedColor::Background as usize {
        let dark = crate::contrast::luminance(default_bg()) < 0.5;
        if super::modelcells::should_drop_bg((set.r, set.g, set.b), dark, false) {
            return query_color(index);
        }
    }
    Some(set)
}

impl TermCore {
    /// Answer the colour queries the last parse raised, each at the place in
    /// the replies it was asked, so a DSR asked after it still comes after.
    pub(super) fn answer_colors(&mut self) {
        let asks = std::mem::take(&mut *self.events.color_asks.lock().unwrap());
        let mut replies = self.events.replies.lock().unwrap();
        // Back to front: inserting a reply moves everything after it.
        for (at, index, format) in asks.into_iter().rev() {
            if let Some(rgb) = drawn(index, self.term.colors()) {
                let at = at.min(replies.len());
                replies.insert_str(at, &format(rgb));
            }
        }
    }
}

#[cfg(test)]
#[path = "modelcolor_tests.rs"]
mod tests;
