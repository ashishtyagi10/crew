//! Per-card field placement for the settings form — the Appearance / Window /
//! Notifications builders behind `form::layout`. Split from `form.rs` for the
//! 200-line cap when the Smoothing picker grew the Appearance card.
use ratatui::layout::Rect;

use super::form::TEXTAREA_ROWS;
use super::Field;

/// Appearance card fields; returns the card height (content + border).
pub(super) fn appearance(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    rects.push((Field::FontFamily, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::FontSize, Rect::new(ix, cy, half, 3)));
    rects.push((Field::PaperGrain, Rect::new(ix + half + 2, cy, half, 3)));
    cy += 3;
    rects.push((Field::Smooth, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::Theme, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    // Directly under Theme, and paired: they are one window, and they are read
    // only while Theme is `auto`. Anywhere else in the card and they read as
    // two unrelated clocks.
    let lh = iw.saturating_sub(2) / 2;
    rects.push((Field::LightFrom, Rect::new(ix, cy, lh, 3)));
    rects.push((Field::LightTo, Rect::new(ix + lh + 2, cy, lh, 3)));
    cy += 3;
    rects.push((Field::Accent, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    let gh = iw.saturating_sub(2) / 2;
    rects.push((Field::Glass, Rect::new(ix, cy, gh, 3)));
    rects.push((Field::Motion, Rect::new(ix + gh + 2, cy, gh, 3)));
    cy += 3;
    rects.push((Field::PaperTexture, Rect::new(ix, cy, iw, 1)));
    cy += 1;
    cy + 1 - y
}

/// Window card fields; returns the card height.
pub(super) fn window(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::NavWidth, Rect::new(ix, cy, half, 3)));
    rects.push((Field::WindowOpacity, Rect::new(ix + half + 2, cy, half, 3)));
    cy += 3;
    for f in [Field::ShowNav, Field::Maximized] {
        rects.push((f, Rect::new(ix, cy, iw, 1)));
        cy += 1;
    }
    cy + 1 - y
}

/// Notifications card fields; returns the card height.
pub(super) fn notifications(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    for f in [
        Field::Notify,
        Field::NotifyAgentDone,
        Field::NotifyBell,
        Field::NotifyExit,
    ] {
        rects.push((f, Rect::new(ix, cy, iw, 1)));
        cy += 1;
    }
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::NotifyMinSecs, Rect::new(ix, cy, half, 3)));
    cy += 3;
    rects.push((
        Field::NotifyPatterns,
        Rect::new(ix, cy, iw, 2 + TEXTAREA_ROWS),
    ));
    cy += 2 + TEXTAREA_ROWS;
    cy + 1 - y
}

/// Content inset inside a card border: x + 2, width − 4.
fn inner(x: u16, w: u16) -> (u16, u16) {
    (x + 2, w.saturating_sub(4))
}
