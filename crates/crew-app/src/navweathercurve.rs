//! The WEATHER card's curve: the next 24 hours as an area chart on the
//! paint layer (the CPU chart's kind), the dot on *now* at the left end
//! and a faint tick where the day ends. Split from `navweathercard`, which
//! draws the words, at the line cap.
use crew_render::Paint;

use crate::navweather::Weather;
use crate::palette::accent;

use crate::navweathercard::TEXT_COL;
/// The curve's rows, added to the card's block when the reading has its hours.
pub const CHART_ROWS: u16 = 2;
/// Row the curve starts on, from the card's rule.
const CHART_OFF: u16 = 3;

/// The hours as `0.0..=1.0` over the day's span — a span floored at four
/// degrees, so a flat day is a flat line at mid-height and not noise
/// stretched to fill the box. Pure.
pub(crate) fn curve(hours: &[i32]) -> Vec<f32> {
    let (lo, hi) = hours
        .iter()
        .fold((i32::MAX, i32::MIN), |(lo, hi), &t| (lo.min(t), hi.max(t)));
    let span = (hi - lo).max(4) as f32;
    let floor = (lo + hi) as f32 / 2.0 - span / 2.0;
    hours
        .iter()
        .map(|&t| ((t as f32 - floor) / span).clamp(0.0, 1.0))
        .collect()
}

/// Where midnight falls across a curve of `len` hours starting at `hour`,
/// as `0.0..1.0` of its width — `None` when the curve starts at midnight
/// (a tick on the left edge would only underline the dot). Pure.
pub(crate) fn midnight_at(hour: u8, len: usize) -> Option<f32> {
    let to_go = 24 - u32::from(hour.min(23));
    (hour > 0 && (to_go as usize) < len).then(|| to_go as f32 / len as f32)
}

/// The curve, in the nav's cell units, shifted to the card at `top`: the
/// area chart under the two text rows, on a hairline it stands on. The
/// dot is on *now* — the left end — and a faint tick says where the day
/// ends: without it the curve's right end reads as the newest reading.
pub(crate) fn weather_paint(w: &Weather, top: u16, cols: u16, aspect: f32) -> Vec<Paint> {
    use crate::plot::area;
    let width = cols.saturating_sub(TEXT_COL + 2);
    if w.hours.len() < 2 || width == 0 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut c = crate::plot::Canvas::new(width, CHART_ROWS, aspect);
    let (cw, ch) = c.size();
    let series = curve(&w.hours);
    let style = area::Style {
        head: false,
        ..area::Style::default()
    };
    if let Some(k) = midnight_at(w.hour, w.hours.len()) {
        c.rect(k * cw, 0.0, 1.0 / 8.0, ch, t.border_normal, 0.55);
    }
    area::draw_styled(&mut c, (0.0, 0.0, cw, ch), &series, accent(), style);
    area::dot(&mut c, 0.25, area::y_at(&series, 0.0, 0.0, ch), accent());
    c.hairline(0.0, ch, cw, t.border_normal, 0.7);
    c.paint()
        .into_iter()
        .map(|p| p.shifted(f32::from(TEXT_COL), f32::from(top + CHART_OFF)))
        .collect()
}

#[cfg(test)]
#[path = "navweathercurve_tests.rs"]
mod tests;
