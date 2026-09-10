//! The WEATHER card: the sky over the place the user named (`/weather`),
//! standing in the nav's glance slot above SERVING. The clock's one-line
//! strip said the numbers; the card has room to say the sky in words and
//! to carry the place on its rule, so `Berlin`'s rain is never read as
//! yours. Under the words, the next 24 hours as a temperature curve on the
//! paint layer (`navweathercurve`). When the layout has no room for it,
//! the strip comes back.
use crew_render::CellView;

use crate::navweather::{glyph, State, Weather};
use crate::palette::accent;

/// Rows the card occupies: rule, now, today, and a one-row gap.
pub const WEATHER_BLOCK: u16 = 4;

/// Column the text (and the curve) starts on, under the rule's own indent.
pub(crate) const TEXT_COL: u16 = 2;
/// Rows the card takes for this state: nothing when off, one quiet row
/// (rule, row, gap) while looking or when the place is not found, and the
/// reading's rows — with the curve only when there is one.
pub(crate) fn block(s: &State) -> u16 {
    match s {
        State::Off => 0,
        State::Looking(_) | State::Missing(_) => QUIET_BLOCK,
        State::Found(w) if w.hours.len() < 2 => WEATHER_BLOCK,
        State::Found(_) => WEATHER_BLOCK + crate::navweathercurve::CHART_ROWS,
    }
}

/// Rows a quiet state's card takes: rule, one muted row, gap.
pub const QUIET_BLOCK: u16 = 3;

/// The card for any state: the reading's card, or the rule with one muted
/// row that says what the card is waiting on.
pub(crate) fn state_cells(s: &State, cols: u16) -> Vec<CellView> {
    let (place, note) = match s {
        State::Off => return Vec::new(),
        State::Found(w) => return weather_cells(w, cols),
        State::Looking(p) => (p.as_str(), "looking up\u{2026}".to_string()),
        State::Missing(p) => ("", format!("nothing for {p}")),
    };
    if cols < 8 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = crate::boxdraw::section_header_key(
        "WEATHER",
        place,
        cols,
        t.border_normal,
        accent(),
        t.text_muted,
        t.page_bg,
    );
    let max_col = cols.saturating_sub(1);
    let room = usize::from(max_col.saturating_sub(TEXT_COL));
    // The fix rides along only where it fits whole: a clipped command is
    // worse than none, and the status line said it already.
    let fix = " \u{2014} /weather <place>";
    let note = match s {
        State::Missing(_) if crate::chatwidth::str_w(&note) + fix.len() <= room => note + fix,
        _ => note,
    };
    let row = crate::chatwidth::clip_w(&note, room)
        .chars()
        .map(|c| (c, t.text_muted, false))
        .collect::<Vec<_>>();
    put(&mut out, 1, row, max_col, t.page_bg);
    out
}

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
