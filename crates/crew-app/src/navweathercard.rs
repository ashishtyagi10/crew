//! The WEATHER card: the sky over the place the user named (`/weather`),
//! standing in the nav's glance slot above SERVING. The clock's one-line
//! strip said the numbers; the card has room to say the sky in words and
//! to carry the place on its rule, so `Berlin`'s rain is never read as
//! yours. Under the words, the next 24 hours as a temperature curve on the
//! paint layer (the CPU chart's kind). When the layout has no room for it,
//! the strip comes back.
use crew_render::{CellView, Paint};

use crate::navweather::{glyph, Weather};
use crate::palette::accent;

/// Rows the card occupies: rule, now, today, and a one-row gap …
pub const WEATHER_BLOCK: u16 = 4;
/// … plus the curve's rows when the reading has its hours.
pub const CHART_ROWS: u16 = 2;
/// Row the curve starts on, from the card's rule.
const CHART_OFF: u16 = 3;

/// Rows this reading's card takes: the curve only when there is one.
pub(crate) fn block(w: &Weather) -> u16 {
    if w.hours.len() < 2 {
        WEATHER_BLOCK
    } else {
        WEATHER_BLOCK + CHART_ROWS
    }
}

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

/// Column the text starts on, under the rule's own indent (as GIT does).
const TEXT_COL: u16 = 2;

/// The sky in a word or two, for a WMO weather code.
pub(crate) fn condition(code: u16) -> &'static str {
    match code {
        0 => "clear",
        1 => "mainly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 | 48 => "fog",
        51..=55 => "drizzle",
        56 | 57 => "freezing drizzle",
        61..=65 => "rain",
        66 | 67 => "freezing rain",
        71..=75 => "snow",
        77 => "snow grains",
        80..=82 => "showers",
        85 | 86 => "snow showers",
        95 => "thunder",
        96 | 99 => "thunder, hail",
        _ => "cloud",
    }
}

/// Render the card: the rule with the place as its key, then
/// `☀ 24° clear` (glyph and reading in the accent, the sky in ink) and
/// `↑27 ↓18 ☂ 10%` (today's range in ink, the arrows and rain muted).
pub(crate) fn weather_cells(w: &Weather, cols: u16) -> Vec<CellView> {
    if cols < 8 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = crate::boxdraw::section_header_key(
        "WEATHER",
        &w.place,
        cols,
        t.border_normal,
        accent(),
        t.text_muted,
        t.page_bg,
    );
    let max_col = cols.saturating_sub(1);
    let room = usize::from(max_col.saturating_sub(TEXT_COL));
    // Row 1: the reading leads and never goes; the words go when the nav is
    // narrow, then the unit.
    let head = format!("{} {}\u{00b0}", glyph(w.code), w.temp);
    let mut now = format!("{head}{} {}", w.unit, condition(w.code));
    if crate::chatwidth::str_w(&now) > room {
        now = format!("{head} {}", condition(w.code));
    }
    if crate::chatwidth::str_w(&now) > room {
        now = format!("{head}{}", w.unit);
    }
    // The reading is the card's one bold thing; the words stand in plain ink.
    let head_n = head.chars().count() + 1;
    let styled: Vec<(char, (u8, u8, u8), bool)> = crate::chatwidth::clip_w(&now, room)
        .chars()
        .enumerate()
        .map(|(i, c)| (c, if i < head_n { accent() } else { t.ink }, i < head_n))
        .collect();
    put(&mut out, 1, styled, max_col, t.page_bg);
    // Row 2: today's range, and the chance of rain when there is one.
    let mut today = vec![('\u{2191}', t.text_muted)];
    let mut push = |s: &str, fg| today.extend(s.chars().map(|c| (c, fg)));
    push(&w.hi.to_string(), t.ink);
    push(" \u{2193}", t.text_muted);
    push(&w.lo.to_string(), t.ink);
    if w.rain > 0 {
        push("  \u{2602}", t.text_muted);
        push(&format!("{}%", w.rain), t.ink);
    }
    let today: Vec<_> = today
        .into_iter()
        .take(room)
        .map(|(c, fg)| (c, fg, false))
        .collect();
    put(&mut out, 2, today, max_col, t.page_bg);
    out
}

fn put(
    out: &mut Vec<CellView>,
    row: u16,
    styled: impl IntoIterator<Item = (char, (u8, u8, u8), bool)>,
    max_col: u16,
    bg: (u8, u8, u8),
) {
    let styled = styled.into_iter().map(|(c, fg, b)| (c, (fg, b)));
    crate::chatwidth::place_row(TEXT_COL, max_col, styled, |x, c, (fg, bold)| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            ..Default::default()
        });
    });
}

#[cfg(test)]
#[path = "navweathercard_tests.rs"]
mod tests;
