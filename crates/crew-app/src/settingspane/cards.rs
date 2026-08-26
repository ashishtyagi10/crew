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
    // `auto`'s settings, under the Theme they belong to and in the order they
    // answer: WHAT it serves per appearance, then WHEN the clock calls it day.
    //
    // The two pairing pickers take the full width, like the Theme picker they
    // qualify, because their values are palette names — `‹ sepia-light ›` is
    // 15 columns and a half-width box holds 14 at an 80-column pane, which
    // clipped the leading chevron and read as a rendering fault. The hours
    // below stay paired: `HH:MM` is five.
    rects.push((Field::ThemeDark, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::ThemeLight, Rect::new(ix, cy, iw, 3)));
    cy += 3;
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
    rects.push((Field::Density, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    // Full width: its legend is longer than a half-width border can carry,
    // and its values are words rather than numbers.
    rects.push((Field::Gradient, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::PaperTexture, Rect::new(ix, cy, iw, 1)));
    cy += 1;
    rects.push((Field::AmbientDrift, Rect::new(ix, cy, iw, 1)));
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

/// Usage card fields; returns the card height. Its own card rather than a
/// tail on NOTIFICATIONS: these are the footer's budget bars, which have
/// nothing to do with being told a command finished.
pub(super) fn usage(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let cy = y + 1;
    let half = iw.saturating_sub(2) / 2;
    rects.push((Field::Budget5h, Rect::new(ix, cy, half, 3)));
    rects.push((Field::Budget7d, Rect::new(ix + half + 2, cy, half, 3)));
    cy + 3 + 1 - y
}

/// Content inset inside a card border: x + 2, width − 4.
fn inner(x: u16, w: u16) -> (u16, u16) {
    (x + 2, w.saturating_sub(4))
}
