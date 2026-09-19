//! The settings form's layout: the pure bento geometry shared by the renderer
//! and the tests (the controls themselves are drawn by [`super::widgets`],
//! re-exported here so callers say `form::input_box`).
//!
//! The columns BALANCE. It used to be `appearance` on the left and everything
//! else stacked on the right, which was true of the card list when it was
//! written and had stopped being true: appearance had grown to twenty fields,
//! so a whole-window form scrolled a 43-row left column past a right column
//! that ran out after 29 and left the rest of the pane empty.
//!
//! So the cards are a LIST now, each one placed into whichever column is
//! shortest, and the number of columns is the one that makes the form
//! shortest — measured, not thresholded, because a third column is only a win
//! while its cards are still wide enough to sit their fields in pairs. On a
//! whole window that is three columns and no scrolling at all.
use ratatui::layout::Rect;

use super::cards::{appearance, canvas, notifications, usage, window};
use super::Field;

/// Columns a card column needs before it is worth having: narrower than this
/// and `pair` stacks every row, which costs more height than the column saves.
const COL_MIN: u16 = 30;
/// The most columns the form will use. Past three, a card is all border.
const MAX_COLS: u16 = 3;
/// Content rows inside the notify-patterns text area.
pub(crate) const TEXTAREA_ROWS: u16 = 4;

/// A card's field builder: places its rects and returns the card height.
type Build = fn(&mut Vec<(Field, Rect)>, u16, u16, u16) -> u16;

/// One bento card: a legend plus the frame the fields are drawn inside.
pub(crate) struct Card {
    pub(crate) title: &'static str,
    pub(crate) rect: Rect,
}

/// Computed form geometry, in virtual rows (y may exceed the pane height).
pub(crate) struct FormLayout {
    pub(crate) cards: Vec<Card>,
    pub(crate) rects: Vec<(Field, Rect)>,
    pub(crate) height: u16,
}

impl FormLayout {
    pub(crate) fn rect_of(&self, f: Field) -> Option<Rect> {
        self.rects.iter().find(|(g, _)| *g == f).map(|&(_, r)| r)
    }
}

/// The cards, longest first — which is both the order a stacked form reads in
/// and what makes the shortest-column rule below pack well (a tall card placed
/// last can only stick out).
const CARDS: [(&str, Build); 5] = [
    ("appearance", appearance),
    ("canvas", canvas),
    ("notifications", notifications),
    ("window", window),
    ("usage", usage),
];

/// Width of one column when the form uses `n` of them: a one-column margin
/// each side and a two-column gutter between.
fn col_width(cols: u16, n: u16) -> u16 {
    cols.saturating_sub(2 + 2 * (n - 1)) / n
}

/// Bento layout: the cards dealt into the column count that makes the form
/// shortest, each card going to whichever column is shortest so far.
pub(crate) fn layout(cols: u16) -> FormLayout {
    let mut best = deal(cols, 1);
    for n in 2..=MAX_COLS {
        if col_width(cols, n) < COL_MIN {
            break;
        }
        let try_n = deal(cols, n);
        if try_n.height < best.height {
            best = try_n;
        }
    }
    best
}

/// Place every card into `n` columns, shortest column first.
fn deal(cols: u16, n: u16) -> FormLayout {
    let w = col_width(cols, n);
    let mut cards = Vec::new();
    // Per column so the rects come out in reading order — down one column,
    // then down the next. Tab reads this order ([`tab_order`]).
    let mut cols_rects: Vec<Vec<(Field, Rect)>> = vec![Vec::new(); usize::from(n)];
    let mut next_y = vec![0u16; usize::from(n)];
    for (title, build) in CARDS {
        let (i, &y) = next_y
            .iter()
            .enumerate()
            .min_by_key(|&(i, &y)| (y, i))
            .expect("at least one column");
        let x = 1 + i as u16 * (w + 2);
        // A blank row between cards in a column, none above the first.
        let y = if y == 0 { 0 } else { y + 1 };
        let h = build(&mut cols_rects[i], x, y, w);
        cards.push(Card {
            title,
            rect: Rect::new(x, y, w, h),
        });
        next_y[i] = y + h;
    }
    FormLayout {
        height: next_y.iter().copied().max().unwrap_or(0),
        rects: cols_rects.concat(),
        cards,
    }
}

/// The focus order at `cols`: the fields in the order the eye meets them.
///
/// WHY derived rather than declared: it used to be a hand-written list beside
/// the `Field` enum, whose own doc said it "follows the eye down the cards".
/// It had stopped — `Paper grain` is drawn third, beside `Font size`, and was
/// tabbed seventeenth; `Nav width` sits in the *window* card and was tabbed
/// fifth, so Tab left the appearance card, crossed the form and came back.
///
/// A second list can always drift from the first. This one cannot, and it is
/// also the only thing that can be right at every width: the form is one, two
/// or three columns of cards depending on what fits, and `pair` stacks its two
/// fields on a narrow card and sits them side by side on a wide one, so the
/// reading order genuinely CHANGES with the width. No static list is correct
/// at all of them.
///
/// [`layout`] already builds the rects in reading order — card by card in
/// placement order, and within a card `pair` pushes left-then-right or
/// top-then-bottom — so this is that order, with any field the layout somehow
/// did not place appended rather than dropped. A field that cannot be reached
/// by Tab is a field that cannot be changed.
pub(crate) fn tab_order(cols: u16) -> Vec<Field> {
    let mut order: Vec<Field> = layout(cols).rects.iter().map(|(f, _)| *f).collect();
    let missed: Vec<Field> = super::fields::FIELDS
        .iter()
        .copied()
        .filter(|f| !order.contains(f))
        .collect();
    order.extend(missed);
    order
}

/// Scroll offset keeping `rect` fully inside a `viewport`-row window over
/// `total` virtual rows (0 when everything fits).
pub(crate) fn scroll_for(rect: Rect, total: u16, viewport: u16) -> u16 {
    if viewport == 0 || total <= viewport {
        return 0;
    }
    (rect.y + rect.height)
        .saturating_sub(viewport)
        .min(rect.y)
        .min(total - viewport)
}

pub(crate) use super::widgets::*;
