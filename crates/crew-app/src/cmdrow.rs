//! One row of the command palette: `› /clear    Clear the pane      Cmd+K`.
//!
//! Three things happen here that a `label + "  " + desc` concatenation cannot
//! do. The **matched characters are marked**, so a fuzzy hit explains itself
//! (`/dmp` finding `/dump` looks like a bug until you can see which letters
//! matched). The **descriptions line up in a column**, so the list is read
//! down rather than scanned. And the **chord**, where one exists, is
//! right-aligned at the far edge, which is how anyone stops needing the
//! palette for that command at all.
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::suggest::MenuItem;

/// Gap between the label column and the descriptions.
const GAP: usize = 2;
/// The label column never takes more than this share of the row: one very long
/// command must not push every description off the card.
const MAX_LABEL_SHARE: usize = 2;
/// Columns of description worth keeping. Below this the row would show a
/// letter and a half, so the chord goes instead.
const MIN_DESC: usize = 8;

/// The columns this list actually needs: the marker, the widest label, the
/// widest description and the widest chord, with the gaps between them.
///
/// The card is drawn at this width rather than at the pane's, because the
/// chord right-aligns to the row's far edge: on a full-window pane that put
/// `Cmd+D` ninety columns from the `/dash` it belongs to, with nothing in
/// between. A list has a measure; past it, it is a band.
pub(crate) fn content_w(items: &[MenuItem]) -> usize {
    let widest = |f: fn(&MenuItem) -> usize| items.iter().map(f).max().unwrap_or(0);
    let label = widest(|i| i.label.chars().count());
    let desc = widest(|i| i.desc.chars().count());
    let key = widest(|i| i.key.map_or(0, |k| k.len()));
    let swatch = widest(|i| i.swatch.len());
    let mut w = 2 + label;
    if swatch > 0 {
        w += GAP + swatch;
    }
    if desc > 0 {
        w += GAP + desc;
    }
    if key > 0 {
        w += GAP + key;
    }
    w
}

/// Width of the label column for `items`: the widest label, bounded by a share
/// of `avail`. Headers are excluded — a section title is not in the column.
///
/// Only rows that HAVE a description set the column. A file path in the
/// attach picker has none, and the longest of them was setting where every
/// agent's description started — half a row away from the agent.
pub(crate) fn label_col(items: &[MenuItem], avail: usize) -> usize {
    let widest = items
        .iter()
        .filter(|i| !i.header && !i.desc.is_empty())
        .map(|i| i.label.chars().count())
        .max()
        .unwrap_or(0);
    widest.min(avail / MAX_LABEL_SHARE)
}

/// Width of the swatch column: the widest swatch in the list, so a row with
/// fewer chips — or none, as `/gradient`'s `off` and `subtle` have — keeps
/// its description in the same column as the rows that carry one.
pub(crate) fn swatch_col(items: &[MenuItem]) -> usize {
    items.iter().map(|i| i.swatch.len()).max().unwrap_or(0)
}

/// The spans of one row, laid out in `avail` columns, its swatch (if any)
/// in a column `swatch_w` wide.
///
/// `hit` positions are character indices into the label; anything past its end
/// is ignored, so a stale match list can never mark a character that is not
/// there.
pub(crate) fn spans(
    item: &MenuItem,
    label_w: usize,
    swatch_w: usize,
    avail: usize,
    dim: Color,
) -> Line<'static> {
    let label_fg = item
        .color
        .map(|(r, g, b)| Color::Rgb(r, g, b))
        .unwrap_or_else(crate::palette::accent_color);
    if item.header {
        // A card can be narrower than its section title, too.
        return Line::from(Span::styled(
            crate::chatwidth::clip_w(&item.label, avail),
            Style::new().fg(dim).add_modifier(Modifier::BOLD),
        ));
    }
    let fg = if item.dim { dim } else { label_fg };
    // A card can be narrower than the command it is listing.
    let label = crate::chatwidth::clip_w(&item.label, avail);
    let mut out: Vec<Span> = Vec::new();
    for (i, c) in label.chars().enumerate() {
        let style = match item.hit.contains(&i) {
            true => Style::new().fg(fg).add_modifier(Modifier::BOLD),
            false => Style::new().fg(fg),
        };
        out.push(Span::styled(c.to_string(), style));
    }
    // Columns drawn so far, so every later gap is padding to a column rather
    // than arithmetic on what was supposed to have been drawn.
    let mut col = label.chars().count();
    // A label wider than the column pushes its own description rather than
    // being cut in half by a column it overflowed.
    let mut desc_col = label_w.max(col) + GAP;
    // The swatch comes first after the label: on a row whose whole subject IS
    // a colour, the colour outranks the sentence describing it. The column is
    // the list's widest swatch, so a row with fewer chips, or none, still
    // starts its description where the others do.
    let sw = swatch_w.max(item.swatch.len());
    if sw > 0 && avail >= desc_col + sw {
        if !item.swatch.is_empty() {
            out.push(Span::raw(" ".repeat(desc_col - col)));
            col = desc_col;
            for chip in &item.swatch {
                let mut style = Style::new().fg(Color::Rgb(chip.fg.0, chip.fg.1, chip.fg.2));
                if let Some((r, g, b)) = chip.bg {
                    style = style.bg(Color::Rgb(r, g, b));
                }
                out.push(Span::styled(chip.c.to_string(), style));
                col += 1;
            }
        }
        desc_col += sw + GAP;
    }
    let room = avail.saturating_sub(desc_col);
    // The chord is the first thing dropped. It is a hint about a row you can
    // already read; the description is what the row is FOR, so a chord never
    // squeezes it below something readable.
    let need = |k: &str| match item.desc.is_empty() {
        true => k.len() + GAP,
        false => k.len() + GAP + MIN_DESC,
    };
    let key = item.key.filter(|k| room >= need(k));
    let desc_w = room.saturating_sub(key.map_or(0, |k| k.len() + GAP));
    if desc_w > 0 && !item.desc.is_empty() {
        out.push(Span::raw(" ".repeat(desc_col - col)));
        // Ellipsized, not cut: "Write the frame's cells to a fi" reads as a
        // rendering fault, where "…to a file" reads as a narrow card. Same
        // rule the nav's prose follows.
        let desc = crate::chatwidth::clip_w(&item.desc, desc_w);
        col = desc_col + crate::chatwidth::str_w(&desc);
        out.push(Span::styled(desc, Style::new().fg(dim)));
    }
    if let Some(k) = key {
        out.push(Span::raw(" ".repeat(avail - col - k.len())));
        out.push(Span::styled(k.to_string(), Style::new().fg(dim)));
    }
    Line::from(out)
}

#[cfg(test)]
#[path = "cmdrowcol_tests.rs"]
mod col_tests;
#[cfg(test)]
#[path = "cmdrow_tests.rs"]
mod tests;
