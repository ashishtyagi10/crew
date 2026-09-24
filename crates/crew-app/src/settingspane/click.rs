//! Where a click lands in the settings form.
//!
//! WHY: the form was keyboard-only. Every control in it LOOKS like something
//! you press — bordered boxes, `[x]` checkboxes, `‹ value ›` pickers with
//! chevrons at both ends, and two buttons that are literally drawn as
//! `[ Save ⌘S ]` — and none of them answered the mouse. A control that looks
//! like a button and ignores a click is worse than one that looks like text.
//!
//! The geometry is read back out of [`super::form::layout`], the same call the
//! renderer draws from, including the scroll offset it blits with. Nothing
//! here re-derives where anything is: a hit test that computes its own layout
//! is a second layout, and it drifts (see `form::tab_order`, which is here for
//! the same reason).
use ratatui::layout::Rect;

use super::labels::value_of;
use super::{form, Field, SettingsPane};

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    /// A field's box: focus it.
    Field(Field),
    /// A `‹ ›` picker's chevron, or a checkbox: focus it and move its value.
    /// `true` steps backwards, which is what the left chevron means.
    Step(Field, bool),
    /// A row of the open font-family list.
    Family(usize),
}

/// The pinned button row's height, below the form's viewport.
const BUTTON_ROWS: u16 = 2;

/// What a click at `(row, col)` lands on, or `None` for the gaps between
/// controls — which still focus the pane, they just do not move anything.
pub(crate) fn hit(p: &SettingsPane, cols: u16, rows: u16, row: u16, col: u16) -> Option<Hit> {
    if cols < 24 || rows < 6 {
        return None; // the pane renders nothing this small
    }
    if row + 1 == rows {
        return buttons(cols, col);
    }
    let lay = form::layout(cols);
    let viewport = rows.saturating_sub(BUTTON_ROWS);
    let off = scroll_off(p, &lay, viewport);
    if row >= viewport {
        return None;
    }
    // The open list is drawn OVER the form, so it answers first — exactly as
    // it is painted.
    if p.family_open {
        if let Some(h) = family_row(p, &lay, off, row, col) {
            return Some(h);
        }
    }
    let virt = row + off;
    let (f, r) = lay
        .rects
        .iter()
        .copied()
        .find(|(_, r)| hits(*r, virt, col))?;
    Some(match chevron(p, f, r, virt, col) {
        Some(back) => Hit::Step(f, back),
        None => Hit::Field(f),
    })
}

/// The scroll the renderer blits with — focus-driven, so it is recomputed the
/// same way rather than remembered.
fn scroll_off(p: &SettingsPane, lay: &form::FormLayout, viewport: u16) -> u16 {
    let tail = Rect::new(0, lay.height.saturating_sub(1), 1, 1);
    let frect = lay.rect_of(p.focused_field()).unwrap_or(tail);
    form::scroll_for(frect, lay.height, viewport)
}

fn hits(r: Rect, row: u16, col: u16) -> bool {
    row >= r.y && row < r.y + r.height && col >= r.x && col < r.x + r.width
}

/// Which chevron of a `‹ value ›` picker was clicked, if either.
///
/// A checkbox counts as its own chevron: a one-row `[x] Label` is a control
/// whose whole point is that pressing it flips it, and a click that only
/// focused it would be a click that visibly did nothing.
fn chevron(p: &SettingsPane, f: Field, r: Rect, row: u16, col: u16) -> Option<bool> {
    if r.height == 1 {
        return Some(false); // a checkbox: clicking it flips it
    }
    let (value, cursor) = value_of(p, f);
    if cursor || !value.starts_with('\u{2039}') {
        return None; // a text field: a click means "put the caret here"
    }
    // The value is drawn one cell inside the border, on the box's middle row.
    if row != r.y + 1 {
        return None;
    }
    let left = r.x + 1;
    let right = left + value.chars().count() as u16 - 1;
    match col {
        c if c == left => Some(true),
        c if c == right => Some(false),
        _ => None,
    }
}

/// Save or Cancel, matching the row `render::buttons` draws.
fn buttons(cols: u16, col: u16) -> Option<Hit> {
    let (save, cancel) = (super::widgets::SAVE, super::widgets::CANCEL);
    let (sw, cw) = (save.chars().count() as u16, cancel.chars().count() as u16);
    let x0 = cols.saturating_sub(sw + 3 + cw + 2);
    match col {
        c if c >= x0 && c < x0 + sw => Some(Hit::Step(Field::Save, false)),
        c if c >= x0 + sw + 3 && c < x0 + sw + 3 + cw => Some(Hit::Step(Field::Cancel, false)),
        _ => None,
    }
}

/// A row of the open family list, matching `dropdown`'s own geometry.
fn family_row(
    p: &SettingsPane,
    lay: &form::FormLayout,
    off: u16,
    row: u16,
    col: u16,
) -> Option<Hit> {
    let anchor = lay.rect_of(Field::FontFamily)?;
    if anchor.y < off {
        return None;
    }
    let y0 = anchor.y - off + anchor.height;
    let n = p.filtered().len();
    // One border row, then the items.
    let i = usize::from(row.checked_sub(y0 + 1)?);
    (i < n && col >= anchor.x && col < anchor.x + anchor.width).then_some(Hit::Family(i))
}
