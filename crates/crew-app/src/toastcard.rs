//! One toast's card: its text laid out to the width it has, and the cells it
//! becomes.
//!
//! Split from [`crate::toast`] for the line cap, along the line between the
//! queue of toasts and the drawing of one.
#[cfg(test)]
#[path = "toastcard_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "toastink_tests.rs"]
mod ink_tests;

use crate::anim::lerp_rgb;
use crate::chatwidth::place_row;
use crew_render::CellView;

/// Widest text a card will hold before clipping (display columns).
pub(crate) const MAX_TEXT_COLS: usize = 46;

/// What one card says: the clipped text, the legend, how many times this
/// exact card has arrived, whether it is an alert, and whether clicking it
/// goes anywhere. Grouped because they travel together and separately would
/// make `card_cells` a wall of positional booleans.
pub(crate) struct CardText<'a> {
    pub(crate) text: &'a str,
    pub(crate) legend: &'a str,
    pub(crate) repeats: usize,
    pub(crate) alert: bool,
    pub(crate) actionable: bool,
}

/// The 3-row fieldset card: legend on the top border, one text row inside.
///
/// `hovered` is the pointer resting on this card — it lights the stroke and,
/// when the card knows a pane (`actionable`), says so in the legend. A click
/// target with no affordance is a secret; the hover is where the card admits
/// it can be clicked.
pub(crate) fn card_cells(c: &CardText, cols: u16, fade: f32, hovered: bool) -> Vec<CellView> {
    let CardText {
        text,
        legend,
        repeats,
        alert,
        actionable,
    } = *c;
    let t = crew_theme::theme();
    let mut border = if alert { t.bell } else { t.border_normal };
    let mut legend_fg = if alert { t.bell } else { t.legend_off };
    // A repeat count is part of what the card says, so it survives the hover
    // rewrite rather than being replaced by it: the reason you are hovering
    // may well be that the card said it happened four times.
    let counted;
    let legend = if repeats > 1 {
        counted = format!("{legend} \u{d7}{repeats}");
        counted.as_str()
    } else {
        legend
    };
    let held;
    let legend = if hovered {
        border = crate::palette::accent();
        legend_fg = border;
        held = if actionable {
            format!("{legend} \u{2192} open")
        } else {
            format!("{legend} \u{2715}")
        };
        held.as_str()
    } else {
        legend
    };
    let border = lerp_rgb(border, t.page_bg, fade);
    let legend_fg = lerp_rgb(legend_fg, t.page_bg, fade);
    let mut cells = crate::boxdraw::titled_card(cols, 3, legend, border, legend_fg, t.page_bg);
    // An ALERT toast keeps its flat bell stroke, and so does a hovered card:
    // both are saying something with colour that a gradient would dilute.
    // Everywhere else a frame's
    // colour is chrome and the gradient is free to take it; here the colour
    // IS the message, and a bell border tinted toward the theme's poles is a
    // warning wearing the same skin as the command menu.
    if !alert && !hovered {
        crate::modernring::quiet(&mut cells, cols, 3, border);
    }
    let fg = lerp_rgb(t.ink, t.page_bg, fade);
    place_row(2, cols - 1, text.chars().map(|c| (c, fg)), |col, c, fg| {
        cells.push(CellView {
            col,
            row: 1,
            c,
            fg,
            bg: t.page_bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    });
    cells
}
