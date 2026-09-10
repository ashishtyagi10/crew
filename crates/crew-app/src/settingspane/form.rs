//! The settings form's layout: the pure two-column geometry shared by the
//! renderer and the tests (the controls themselves are drawn by
//! [`super::widgets`], re-exported here so callers say `form::input_box`).
//!
//! The left column is `appearance` alone because it is the tall one; the
//! right stacks `window`, `notifications` and `usage`, which together
//! roughly match it.
use ratatui::layout::Rect;

use super::cards::{appearance, notifications, usage, window};
use super::Field;

/// Pane width below which the two card columns stack vertically.
pub(crate) const STACK_BELOW: u16 = 64;
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

/// Bento layout: two columns when the pane is wide enough (Appearance left;
/// Window + Notifications + Usage right), otherwise one stacked column.
pub(crate) fn layout(cols: u16) -> FormLayout {
    let mut rects = Vec::new();
    let mut cards = Vec::new();
    // Place one card and report the row after it, so adding a card is one
    // line rather than the five-line push-and-measure dance this repeated.
    let mut place = |rects: &mut Vec<(Field, Rect)>, title, build: Build, x, y, w| {
        let h = build(rects, x, y, w);
        cards.push(Card {
            title,
            rect: Rect::new(x, y, w, h),
        });
        y + h
    };
    if cols >= STACK_BELOW {
        let col_w = (cols - 4) / 2; // 1-col margins + 2-col gutter
        let (lx, rx) = (1, 1 + col_w + 2);
        let left = place(&mut rects, "appearance", appearance, lx, 0, col_w);
        let mut right = place(&mut rects, "window", window, rx, 0, col_w);
        for (title, build) in [("notifications", notifications as Build), ("usage", usage)] {
            right = place(&mut rects, title, build, rx, right + 1, col_w);
        }
        FormLayout {
            cards,
            rects,
            height: left.max(right),
        }
    } else {
        let w = cols.saturating_sub(2);
        let mut y = 0;
        for (title, build) in [
            ("appearance", appearance as Build),
            ("window", window),
            ("notifications", notifications),
            ("usage", usage),
        ] {
            y = place(&mut rects, title, build, 1, y, w) + 1;
        }
        FormLayout {
            cards,
            rects,
            height: y - 1,
        }
    }
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
