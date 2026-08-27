//! The colours a value picker draws beside the value it is offering.
//!
//! `/gradient aurora` and `/theme dark` name colours. Reading their names off
//! a list and pressing Enter to find out what they look like is the picker
//! failing at the one thing a picker is for, so each row carries the thing it
//! stands for: the gradient as a four-cell ramp between its poles, a theme
//! mode as one chip per palette in its rotation — the page it puts up, with
//! that palette's accent across the top half.
use crew_theme::{RandomMode, ThemeId, ALL_THEMES};

/// One cell of a swatch: a glyph in `fg` over `bg`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Chip {
    pub c: char,
    pub fg: (u8, u8, u8),
    pub bg: Option<(u8, u8, u8)>,
}

/// Cells a gradient ramp takes. Four is enough to read as a direction and
/// short enough to leave the description its column.
const RAMP: usize = 4;

fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    (mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

/// The swatch for `value` of `cmd`, empty when the value is not a colour.
pub(crate) fn for_value(cmd: &str, value: &str) -> Vec<Chip> {
    match cmd {
        "/gradient" => crew_theme::gradients::by_name(value)
            .map(|(a, b)| {
                (0..RAMP)
                    .map(|i| Chip {
                        c: '\u{2588}',
                        fg: lerp(a, b, i as f32 / (RAMP - 1) as f32),
                        bg: None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        // Both halves of what `/theme` accepts: a rotation mode shows its
        // whole pool, a pinned palette name shows itself.
        "/theme" => match crew_theme::parse_selection(value) {
            Some(crew_theme::Selection::Mode(m)) => pool_chips(m),
            Some(crew_theme::Selection::Fixed(id)) => vec![chip_of(id)],
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// One chip per palette this mode rotates through, in `ALL_THEMES` order.
fn pool_chips(m: RandomMode) -> Vec<Chip> {
    ALL_THEMES
        .iter()
        .filter(|&&id| m.in_pool(id))
        .map(|&id| chip_of(id))
        .collect()
}

/// A palette as one cell: its page underneath, its accent across the top half.
/// Two colours in one column is what makes a dark pool's chips tell each other
/// apart — every one of their pages is nearly black.
fn chip_of(id: ThemeId) -> Chip {
    let t = id.theme();
    Chip {
        c: '\u{2580}',
        fg: t.accent_default,
        bg: Some(t.page_bg),
    }
}

#[cfg(test)]
#[path = "swatch_tests.rs"]
mod tests;
