//! Per-card field placement for the settings form — the Appearance / Window /
//! Notifications builders behind `form::layout`. Split from `form.rs` for the
//! 200-line cap when the Smoothing picker grew the Appearance card.
use ratatui::layout::Rect;

use super::fit::min_cols;
use super::form::TEXTAREA_ROWS;
use super::Field;

/// Place `a` and `b` on one row when both fit half of `iw`, otherwise stack
/// them. Returns the rows consumed (3 paired, 6 stacked).
///
/// The pairing decision belongs to the WIDTH, not to the field (see
/// [`super::fit`]): a field pinned to full width by hand is correct at the
/// pane width someone happened to test and wasteful at every other. Both
/// halves are checked, since a row is only as pairable as its wider half.
fn lone(rects: &mut Vec<(Field, Rect)>, ix: u16, iw: u16, cy: u16, f: Field) {
    // A field with nothing beside it still wants to look like the paired ones
    // — but only while a half actually carries it. `Min secs` sat at a hard
    // half and clipped its own legend below about 70 columns.
    let half = iw.saturating_sub(2) / 2;
    let w = if half >= min_cols(f) { half } else { iw };
    rects.push((f, Rect::new(ix, cy, w, 3)));
}

fn pair(rects: &mut Vec<(Field, Rect)>, ix: u16, iw: u16, cy: u16, a: Field, b: Field) -> u16 {
    let half = iw.saturating_sub(2) / 2;
    if half >= min_cols(a) && half >= min_cols(b) {
        rects.push((a, Rect::new(ix, cy, half, 3)));
        rects.push((b, Rect::new(ix + half + 2, cy, half, 3)));
        return 3;
    }
    rects.push((a, Rect::new(ix, cy, iw, 3)));
    rects.push((b, Rect::new(ix, cy + 3, iw, 3)));
    6
}

/// Appearance card fields; returns the card height (content + border).
pub(super) fn appearance(rects: &mut Vec<(Field, Rect)>, x: u16, y: u16, w: u16) -> u16 {
    let (ix, iw) = inner(x, w);
    let mut cy = y + 1;
    rects.push((Field::FontFamily, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    cy += pair(rects, ix, iw, cy, Field::FontSize, Field::PaperGrain);
    rects.push((Field::Smooth, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    rects.push((Field::Theme, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    // `auto`'s settings, under the Theme they belong to and in the order they
    // answer: WHAT it serves per appearance, then WHEN the clock calls it day.
    //
    // These two used to be pinned full-width by hand, because their values are
    // palette names — `‹ sepia-light ›` is 15 columns, a half-width box held
    // 14 at an 80-column pane, and the clipped leading chevron read as a
    // rendering fault. `pair` now takes that decision from the width itself,
    // so they stack where they must and sit side by side on a pane with room.
    cy += pair(rects, ix, iw, cy, Field::ThemeDark, Field::ThemeLight);
    cy += pair(rects, ix, iw, cy, Field::LightFrom, Field::LightTo);
    rects.push((Field::Accent, Rect::new(ix, cy, iw, 3)));
    cy += 3;
    cy += pair(rects, ix, iw, cy, Field::Glass, Field::Motion);
    cy += pair(rects, ix, iw, cy, Field::Density, Field::Contrast);
    rects.push((Field::ShapeCues, Rect::new(ix, cy, iw, 3)));
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
    cy += pair(rects, ix, iw, cy, Field::NavWidth, Field::WindowOpacity);
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
    lone(rects, ix, iw, cy, Field::NotifyMinSecs);
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
    let rows = pair(rects, ix, iw, cy, Field::Budget5h, Field::Budget7d);
    cy + rows + 1 - y
}

/// Content inset inside a card border: x + 2, width − 4.
fn inner(x: u16, w: u16) -> (u16, u16) {
    (x + 2, w.saturating_sub(4))
}
