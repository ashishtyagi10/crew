//! What colour a pane's frame and its legend are drawn in — the one place the
//! card's states are ranked against each other.
//!
//! Split from [`super::panecard`] (which was at its line ceiling) when
//! BROADCAST joined them. The order is by urgency, and each rank is a thing
//! about to happen or happening to your keystrokes:
//!
//! 1. **a drop target** — a card is being carried over this one and a swap is
//!    one release away.
//! 2. **broadcast** — what you type goes to every terminal, this one included.
//!    It wore one `»` on the top border and nothing else, so a mode that
//!    redirects every keystroke looked exactly like a mode that does not: it
//!    was reported as "I typed in one pane and it showed up in another". Now
//!    the whole frame says it, on every pane it is true of, for as long as it
//!    is true — the focus brackets still mark which one you are in.
//! 3. **focus** — the pane your keys reach when nothing is redirecting them.
use crate::panecard::Bar;

/// `(border, legend)` for this card.
pub(crate) fn stroke(b: &Bar, hue: (u8, u8, u8)) -> ((u8, u8, u8), (u8, u8, u8)) {
    let t = crew_theme::theme();
    // A card carried over this one lights its whole frame: the drop lands
    // here, and a swap is worth saying before it happens (see `panedrag`).
    if crate::panedrag::is_drop_target(b.index.unwrap_or(0) as u16) {
        return (crate::palette::accent(), crate::palette::accent());
    }
    let legend = match b.focused {
        true => hue,
        false => crate::anim::lerp_rgb(hue, t.legend_off, 0.55),
    };
    if b.broadcast {
        return (t.broadcast, legend);
    }
    let border = match b.focused {
        true => crate::panecardglow::focused_stroke(t),
        false => t.border_normal,
    };
    (border, legend)
}

#[cfg(test)]
#[path = "panecardink_tests.rs"]
mod tests;
