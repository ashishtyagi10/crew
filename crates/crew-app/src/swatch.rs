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
            Some(crew_theme::Selection::Fixed(id)) => palette_chips(id),
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// A `#rrggbb` value as its own chip — what the Accent field holds. `None`
/// for anything that is not a full six-digit hex colour, including the empty
/// field (which means "follow the theme" and has no one colour to show).
pub(crate) fn hex_chip(value: &str) -> Option<Chip> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(Chip {
        c: '\u{2588}',
        fg: (byte(0)?, byte(2)?, byte(4)?),
        bg: None,
    })
}

/// One chip per palette this mode rotates through, in `ALL_THEMES` order.
fn pool_chips(m: RandomMode) -> Vec<Chip> {
    ALL_THEMES
        .iter()
        .filter(|&&id| m.in_pool(id))
        .map(|&id| chip_of(id, |t| t.accent_default))
        .collect()
}

/// A palette as one cell: its page underneath, `over` across the top half.
/// Two colours in one column is what makes a dark pool's chips tell each other
/// apart — every one of their pages is nearly black.
fn chip_of(id: ThemeId, over: Face) -> Chip {
    let t = id.theme();
    Chip {
        c: '\u{2580}',
        fg: over(t),
        bg: Some(t.page_bg),
    }
}

/// One colour read off a palette — a face of it, for the strip below.
type Face = fn(&crew_theme::Theme) -> (u8, u8, u8);

/// The half of a palette that tells it apart from its neighbours, in the
/// order they matter.
///
/// A pinned palette used to show ONE chip — its page with its accent on top —
/// which is the same amount of information the rotation modes get for each
/// member of their pool. That is enough to pick a *pool* out of four and far
/// too little to pick a *palette* out of twelve: the dark pool's pages are all
/// nearly black, and their accents are the one colour a user is most likely to
/// have overridden anyway.
///
/// So a named palette shows its hand: the ink it writes in, its accent, and
/// the four ANSI slots every program in a pane is about to paint with. Red and
/// green are the ones that carry meaning (a failure, a passing test), and
/// yellow and blue are where two palettes with the same page most visibly
/// disagree. All eight colours ride that palette's own page, so the strip is a
/// small picture of what the screen will look like rather than a list of
/// values.
const FACES: [Face; 6] = [
    |t| t.ink,
    |t| t.accent_default,
    |t| t.ansi[1], // red
    |t| t.ansi[2], // green
    |t| t.ansi[3], // yellow
    |t| t.ansi[4], // blue
];

/// The strip a named palette shows: [`FACES`], each over that palette's page.
fn palette_chips(id: ThemeId) -> Vec<Chip> {
    FACES.iter().map(|f| chip_of(id, *f)).collect()
}

#[cfg(test)]
#[path = "swatch_tests.rs"]
mod tests;
