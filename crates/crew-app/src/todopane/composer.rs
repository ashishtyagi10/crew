//! The todo composer: the bordered card at the bottom of the pane you type
//! into, and the wrap arithmetic that decides how tall it is.
//!
//! Split out of `render` so the list's own layout and the box you type into
//! are separate things to read; `render::cells` calls [`cells`] last, over
//! the rows [`height`] reserved for it.
use crew_render::CellView;

use super::render::{cell, content, wrap_ranges};
use super::{duedate, TodoPane};

/// Composer rows at this pane size: a bordered card whose interior grows
/// with the wrapped input ([`input_lines`], capped by [`composer_cap`]), or
/// a single bare prompt row on very short panes.
pub(crate) fn height(p: &TodoPane, cols: u16, rows: u16) -> u16 {
    let cols = content(cols);

    if rows >= 6 {
        2 + input_lines(p, cols).len().min(cap(rows)) as u16
    } else {
        1
    }
}

/// Most interior (text) rows the composer may take before it stops growing
/// and tail-follows by line — keeps some list rows visible on short panes.
fn cap(rows: u16) -> usize {
    (rows.saturating_sub(4) as usize).clamp(1, 4)
}

/// The composer input wrapped at the card's interior width: absolute char
/// ranges into `p.input`, always at least one (possibly empty) line. The
/// budget leaves the cursor column free, so a full line never clips `▏`.
pub(crate) fn input_lines(p: &TodoPane, cols: u16) -> Vec<(usize, usize)> {
    let cols = content(cols);

    let chars: Vec<char> = p.input.chars().collect();
    let w = (cols.saturating_sub(6)).max(1) as usize;
    wrap_ranges(&chars, w, w)
}

/// The bordered composer at the bottom: legend carries live feedback (a
/// recognised due date, an edit in progress, the active filter), the
/// interior the `❯ input▏` prompt — wrapping onto further rows as the text
/// fills the width ([`input_lines`]), the last rows kept once the cap is
/// hit (editing happens at the end) — with the date fragment and `@tags`
/// tinted as they're typed. Very short panes get one bare tail-follow row.
pub(crate) fn cells(out: &mut Vec<CellView>, p: &TodoPane, cols: u16, rows: u16) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let ch = height(p, cols, rows);
    let top = rows - ch;
    let now = duedate::now_local();
    let hit = duedate::find(&p.input, now);

    let chars: Vec<char> = p.input.chars().collect();
    let in_date = |i: usize| hit.as_ref().is_some_and(|h| i >= h.start && i < h.end);
    // Each span carries its tag's own color (one hash per tag per redraw),
    // so typing `@crew` tints live in the same color its row chip will get.
    let tag_tints: Vec<(usize, usize, (u8, u8, u8))> = tag_spans(&chars)
        .into_iter()
        .map(|(s, e)| {
            let name: String = chars[s + 1..e].iter().collect();
            (s, e, crew_theme::tag_color(&name, t))
        })
        .collect();
    let in_tag = |i: usize| {
        tag_tints
            .iter()
            .find(|&&(s, e, _)| i >= s && i < e)
            .map(|&(_, _, fg)| fg)
    };
    let style = |i: usize| {
        if in_date(i) {
            (accent, true)
        } else if let Some(fg) = in_tag(i) {
            (fg, false)
        } else {
            (t.ink, false)
        }
    };

    let cursor = p.cursor.min(chars.len());
    if ch == 1 {
        // Bare row: no room to grow — follow the CURSOR, not the tail, so a
        // mid-string edit stays under the eye.
        out.push(cell(0, top, '\u{276f}', accent, true)); // ❯
        let (text_x, max) = (2u16, cols);
        let avail = (max.saturating_sub(text_x)).saturating_sub(1) as usize;
        let mut start = cursor;
        let mut used = 0;
        while start > 0 {
            let w = crate::chatwidth::char_w(chars[start - 1]);
            if used + w > avail {
                break;
            }
            used += w;
            start -= 1;
        }
        let styled = chars[start..]
            .iter()
            .enumerate()
            .map(|(j, &c)| (c, style(start + j)));
        crate::chatwidth::place_row(text_x, max, styled, |x, c, (fg, bold)| {
            out.push(cell(x, top, c, fg, bold))
        });
        let bar_x = text_x + used as u16;
        if bar_x < max {
            out.push(cell(bar_x, top, '\u{258f}', accent, false)); // ▏
        }
        return;
    }

    let legend = if p.done_view {
        match &p.filter {
            Some(f) => format!("done @{f}"),
            None => "done".to_string(),
        }
    } else if p.editing.is_some() {
        "edit".to_string()
    } else if let Some(h) = &hit {
        format!("due {}", duedate::label_naive(h.due, h.has_time, now))
    } else if let Some(f) = &p.filter {
        format!("@{f}")
    } else {
        "new".to_string()
    };
    // Color follows what the legend actually says: due dates in accent, an
    // active `@filter` in that tag's color, edit/new in the resting tone.
    let legend_fg = if p.done_view {
        match &p.filter {
            Some(f) => crew_theme::tag_color(f, t),
            None => t.legend_off,
        }
    } else if p.editing.is_some() {
        t.legend_off
    } else if hit.is_some() {
        accent
    } else if let Some(f) = &p.filter {
        crew_theme::tag_color(f, t)
    } else {
        t.legend_off
    };
    for mut c in
        crate::boxdraw::titled_card(cols, ch, &legend, t.border_normal, legend_fg, t.page_bg)
    {
        c.row += top;
        out.push(c);
    }

    let (text_x, max) = (4u16, cols - 1);
    out.push(cell(2, top + 1, '\u{276f}', accent, true)); // ❯
    if p.input.is_empty() {
        let hint = if p.done_view {
            "filter with @project \u{b7} esc leaves"
        } else {
            "type a todo"
        };
        let styled = hint.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(text_x, max, styled, |x, c, ()| {
            out.push(cell(x, top + 1, c, t.text_muted, false))
        });
        return;
    }

    // Show `ch - 2` wrapped lines, the window anchored so the CURSOR's line
    // is always visible (with the cursor at the end this is the old
    // tail-follow; mid-draft edits keep their line on screen instead).
    let lines = input_lines(p, cols);
    let visible = (ch - 2) as usize;
    let cur_line = lines
        .iter()
        .position(|&(_, e)| cursor <= e)
        .unwrap_or(lines.len() - 1);
    let skip = cur_line
        .saturating_sub(visible.saturating_sub(1))
        .min(lines.len().saturating_sub(visible));
    let (mut bar_x, mut bar_row) = (text_x, top + 1);
    for (vi, &(s, e)) in lines[skip..].iter().take(visible).enumerate() {
        let r = top + 1 + vi as u16;
        let styled = chars[s..e]
            .iter()
            .enumerate()
            .map(|(j, &c)| (c, style(s + j)));
        let end_x = crate::chatwidth::place_row(text_x, max, styled, |x, c, (fg, bold)| {
            out.push(cell(x, r, c, fg, bold))
        });
        if skip + vi == cur_line {
            let w: usize = chars[s..cursor.min(e).max(s)]
                .iter()
                .map(|&c| crate::chatwidth::char_w(c))
                .sum();
            bar_x = (text_x + w as u16).min(max.saturating_sub(1));
            bar_row = r;
        } else if cursor > e && skip + vi == lines.len().saturating_sub(1) {
            // Cursor past the last visible glyph (trailing-space wrap gap):
            // park the bar after the line's text.
            bar_x = end_x;
            bar_row = r;
        }
    }
    // Drawn last, over the glyph it sits on — a beam at the cursor. The
    // wrap budget leaves the end column free ([`input_lines`]).
    out.push(cell(bar_x, bar_row, '\u{258f}', accent, false)); // ▏
}

/// Char ranges of every `@tag` token (length ≥ 2) in the composer.
fn tag_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        if chars[start] == '@' && i - start > 1 {
            spans.push((start, i));
        }
    }
    spans
}
