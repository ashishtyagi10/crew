//! Rendering (and the matching click geometry) for the todo pane: the item
//! list from the top, the `@project` popup and the bordered composer at the
//! bottom. All layout arithmetic lives here so `cells` and [`click_at`] can
//! never disagree about what sits on a row.
use crew_render::CellView;

use super::item::TodoItem;
use super::{duedate, TodoPane};

/// Column where the `[ ]` checkbox starts; the title follows two past it.
const BOX_COL: u16 = 2;
const TITLE_COL: u16 = 6;
/// Cap on visible popup rows (incl. its 2 border rows).
const POPUP_MAX: u16 = 8;

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bold: bool) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: crew_theme::theme().page_bg,
        bold,
        italic: false,
    }
}

/// Composer rows at this pane size: a bordered card whose interior grows
/// with the wrapped input ([`input_lines`], capped by [`composer_cap`]), or
/// a single bare prompt row on very short panes.
pub(crate) fn composer_h(p: &TodoPane, cols: u16, rows: u16) -> u16 {
    if rows >= 6 {
        2 + input_lines(p, cols).len().min(composer_cap(rows)) as u16
    } else {
        1
    }
}

/// Most interior (text) rows the composer may take before it stops growing
/// and tail-follows by line — keeps some list rows visible on short panes.
fn composer_cap(rows: u16) -> usize {
    (rows.saturating_sub(4) as usize).clamp(1, 4)
}

/// The composer input wrapped at the card's interior width: absolute char
/// ranges into `p.input`, always at least one (possibly empty) line. The
/// budget leaves the cursor column free, so a full line never clips `▏`.
fn input_lines(p: &TodoPane, cols: u16) -> Vec<(usize, usize)> {
    let chars: Vec<char> = p.input.chars().collect();
    let w = (cols.saturating_sub(6)).max(1) as usize;
    wrap_ranges(&chars, w, w)
}

/// The dim info row shown above the list while a `@project` filter is on.
fn header_h(p: &TodoPane) -> u16 {
    u16::from(p.filter.is_some())
}

/// Rows the open tag popup occupies (0 when closed or the pane is short).
pub(crate) fn popup_h(p: &TodoPane, rows: u16) -> u16 {
    match &p.tagmenu {
        Some(m) if rows >= 10 && !m.matches.is_empty() => {
            crate::cmdmenu::menu_rows(m.matches.len()).min(POPUP_MAX)
        }
        _ => 0,
    }
}

/// Rows left for the item list.
pub(crate) fn list_height(p: &TodoPane, cols: u16, rows: u16) -> u16 {
    rows.saturating_sub(composer_h(p, cols, rows) + popup_h(p, rows) + header_h(p))
}

/// Mirror of [`place_right`]'s arithmetic without the cells: the next free
/// slot after a width-`w` chip ending at `end` (or `end` unchanged when the
/// chip would reach the title zone and goes unplaced).
fn place_w(end: u16, w: u16) -> u16 {
    let start = end.saturating_sub(w);
    if start <= TITLE_COL {
        end
    } else {
        start.saturating_sub(2)
    }
}

/// Column past the title's first line — where the right-side chips (due,
/// `@tag`, `✗`) begin, one-column gap included.
fn first_line_max(it: &TodoItem, cols: u16, now_ms: u64) -> u16 {
    let mut right = cols.saturating_sub(4);
    if let Some(due) = it.due_ms {
        let lbl = duedate::label(due, it.due_has_time, now_ms);
        right = place_w(right, crate::chatwidth::str_w(&lbl) as u16);
    }
    if let Some(tag) = &it.project {
        right = place_w(right, crate::chatwidth::str_w(&format!("@{tag}")) as u16);
    }
    right + 1
}

/// The title's wrapped lines as char-index ranges: greedy word wrap, the
/// first line stopping where the chips begin, continuation lines spanning
/// the pane. Always at least one range, so every item owns a row.
fn title_lines(it: &TodoItem, cols: u16, now_ms: u64) -> Vec<(usize, usize)> {
    let chars: Vec<char> = it.title.chars().collect();
    let w0 = (first_line_max(it, cols, now_ms).saturating_sub(TITLE_COL)).max(1) as usize;
    let wc = (cols.saturating_sub(2 + TITLE_COL)).max(1) as usize;
    wrap_ranges(&chars, w0, wc)
}

/// Greedy word wrap over `chars` into (start, end) char ranges: the first
/// line `w0` cells wide, continuations `wc`. Always at least one range.
fn wrap_ranges(chars: &[char], w0: usize, wc: usize) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        let budget = if lines.is_empty() { w0 } else { wc };
        let fit = crate::chatwidth::fit_end(chars, start, budget);
        if fit >= chars.len() {
            lines.push((start, chars.len()));
            return lines;
        }
        // Break on the last space inside the window when the cut would land
        // mid-word; a single over-long word hard-breaks.
        let cut = chars[start..fit]
            .iter()
            .rposition(|c| c.is_whitespace())
            .map(|i| start + i)
            .filter(|&i| i > start && !chars[fit].is_whitespace())
            .unwrap_or(fit);
        lines.push((start, cut));
        start = cut;
        while start < chars.len() && chars[start].is_whitespace() {
            start += 1;
        }
        if start >= chars.len() {
            return lines;
        }
    }
}

/// Rows item `it` occupies at this pane width.
pub(crate) fn item_h(it: &TodoItem, cols: u16, now_ms: u64) -> u16 {
    title_lines(it, cols, now_ms).len() as u16
}

/// What a click on pane-content cell (`row`, `col`) means.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TodoClick {
    /// The `[ ]` checkbox of the visible row at this display index.
    Toggle(usize),
    /// The `✗` at the row's end.
    Delete(usize),
    /// Anywhere else on an item row.
    Select(usize),
    /// The composer area — refocus it.
    Composer,
}

/// Map a content-cell click to its action; `None` falls through to the
/// app's normal focus path.
pub(crate) fn click_at(
    p: &TodoPane,
    row: u16,
    col: u16,
    cols: u16,
    rows: u16,
) -> Option<TodoClick> {
    if row >= rows.saturating_sub(composer_h(p, cols, rows)) {
        return Some(TodoClick::Composer);
    }
    let header = header_h(p);
    let bottom = header + list_height(p, cols, rows);
    if row < header || row >= bottom {
        return None;
    }
    let now_ms = crate::chattime::unix_now_ms();
    let order = p.order();
    let mut top = header;
    for (di, &idx) in order.iter().enumerate().skip(p.scroll) {
        if top >= bottom {
            break;
        }
        let h = item_h(&p.items[idx], cols, now_ms);
        if row < top + h {
            // Continuation rows carry no checkbox or ✗ — they just select.
            return Some(if row > top {
                TodoClick::Select(di)
            } else if (BOX_COL..BOX_COL + 3).contains(&col) {
                TodoClick::Toggle(di)
            } else if col >= cols.saturating_sub(3) {
                TodoClick::Delete(di)
            } else {
                TodoClick::Select(di)
            });
        }
        top += h;
    }
    None
}

/// Render the pane's `cols × rows` content grid.
pub(crate) fn cells(p: &TodoPane, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 8 || rows < 2 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let now_ms = crate::chattime::unix_now_ms();
    let order = p.order();
    let mut out = Vec::new();

    if let Some(f) = &p.filter {
        let n = order.len();
        let line = format!("@{f} · {n} item{}", if n == 1 { "" } else { "s" });
        let styled = line.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(BOX_COL, cols, styled, |x, c, ()| {
            out.push(cell(x, 0, c, t.text_muted, false))
        });
    }

    let header = header_h(p);
    let lh = list_height(p, cols, rows) as usize;
    let bottom = header + lh as u16;
    let mut row = header;
    for (di, &idx) in order.iter().enumerate().skip(p.scroll) {
        if row >= bottom {
            break;
        }
        let selected = p.sel == Some(di);
        row_cells(&mut out, p, idx, row, cols, bottom, selected, now_ms);
        row += item_h(&p.items[idx], cols, now_ms);
    }
    if order.is_empty() && lh >= 2 {
        for (i, hint) in [
            "no todos",
            "type one below — try: pay rent tomorrow 5pm @home",
        ]
        .iter()
        .enumerate()
        {
            let row = header + (lh as u16 / 2).saturating_sub(1) + i as u16;
            let styled = hint.chars().map(|c| (c, ()));
            crate::chatwidth::place_row(BOX_COL, cols, styled, |x, c, ()| {
                out.push(cell(x, row, c, t.text_muted, false))
            });
        }
    }

    let ph = popup_h(p, rows);
    if let (Some(m), true) = (&p.tagmenu, ph > 0) {
        let items: Vec<crate::suggest::MenuItem> = m
            .matches
            .iter()
            .map(|tag| crate::suggest::MenuItem {
                label: format!("@{tag}"),
                desc: String::new(),
                fill: String::new(),
                submit: false,
                header: false,
                dim: false,
                needs: None,
            })
            .collect();
        let top = rows - composer_h(p, cols, rows) - ph;
        for mut c in crate::cmdmenu::menu_card("projects", &items, m.sel, cols, ph) {
            c.row += top;
            out.push(c);
        }
    }

    composer_cells(&mut out, p, cols, rows);
    out
}

/// One item: `› [ ] title … @tag due ✗` on its first row, the title
/// wrapping onto full-width continuation rows below ([`title_lines`]);
/// rows at or past `bottom` are clipped.
fn row_cells(
    out: &mut Vec<CellView>,
    p: &TodoPane,
    idx: usize,
    row: u16,
    cols: u16,
    bottom: u16,
    selected: bool,
    now_ms: u64,
) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let it = &p.items[idx];
    let ink = t.ink;

    if selected {
        out.push(cell(0, row, '\u{203a}', accent, true)); // ›
    }
    for (i, c) in "[ ]".chars().enumerate() {
        out.push(cell(BOX_COL + i as u16, row, c, ink, selected));
    }

    // Right side, laid right-to-left: ✗, due, @tag.
    let del_col = cols - 2;
    out.push(cell(
        del_col,
        row,
        '\u{2717}', // ✗
        if selected { t.ink } else { t.text_muted },
        false,
    ));
    let mut right = del_col.saturating_sub(2);
    if let Some(due) = it.due_ms {
        let lbl = duedate::label(due, it.due_has_time, now_ms);
        let overdue = due <= now_ms;
        let today = duedate::days_from_now(due, now_ms) == Some(0);
        let fg = if overdue {
            t.bell
        } else if today {
            t.status_fg
        } else {
            t.text_muted
        };
        right = place_right(out, &lbl, right, row, fg, overdue);
    }
    if let Some(tag) = &it.project {
        let chip = format!("@{tag}");
        right = place_right(out, &chip, right, row, accent, false);
    }
    // `right` is the next free slot two left of the leftmost right-side
    // text; the title keeps a one-column gap before that text and wraps
    // onto full-width rows below.
    debug_assert_eq!(right + 1, first_line_max(it, cols, now_ms));
    let chars: Vec<char> = it.title.chars().collect();
    for (li, &(s, e)) in title_lines(it, cols, now_ms).iter().enumerate() {
        let r = row + li as u16;
        if r >= bottom {
            break;
        }
        let max = if li == 0 { right + 1 } else { cols - 2 };
        let styled = chars[s..e].iter().map(|&c| (c, ()));
        crate::chatwidth::place_row(TITLE_COL, max, styled, |x, c, ()| {
            out.push(cell(x, r, c, ink, selected))
        });
    }
}

/// Place `s` ending at `end` (exclusive of the following gap) on `row`;
/// returns the column two left of where it started (the next slot).
fn place_right(
    out: &mut Vec<CellView>,
    s: &str,
    end: u16,
    row: u16,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let w = crate::chatwidth::str_w(s) as u16;
    let start = end.saturating_sub(w);
    if start <= TITLE_COL {
        return end;
    }
    let styled = s.chars().map(|c| (c, ()));
    crate::chatwidth::place_row(start, end, styled, |x, c, ()| {
        out.push(cell(x, row, c, fg, bold))
    });
    start.saturating_sub(2)
}

/// The bordered composer at the bottom: legend carries live feedback (a
/// recognised due date, an edit in progress, the active filter), the
/// interior the `❯ input▏` prompt — wrapping onto further rows as the text
/// fills the width ([`input_lines`]), the last rows kept once the cap is
/// hit (editing happens at the end) — with the date fragment and `@tags`
/// tinted as they're typed. Very short panes get one bare tail-follow row.
fn composer_cells(out: &mut Vec<CellView>, p: &TodoPane, cols: u16, rows: u16) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let ch = composer_h(p, cols, rows);
    let top = rows - ch;
    let now = duedate::now_local();
    let hit = duedate::find(&p.input, now);

    let chars: Vec<char> = p.input.chars().collect();
    let in_date = |i: usize| hit.as_ref().is_some_and(|h| i >= h.start && i < h.end);
    let tag_spans = tag_spans(&chars);
    let in_tag = |i: usize| tag_spans.iter().any(|&(s, e)| i >= s && i < e);
    let style = |i: usize| {
        if in_date(i) {
            (accent, true)
        } else if in_tag(i) {
            (accent, false)
        } else {
            (t.ink, false)
        }
    };

    if ch == 1 {
        // Bare row: no room to grow, tail-follow the last chars that fit.
        out.push(cell(0, top, '\u{276f}', accent, true)); // ❯
        let (text_x, max) = (2u16, cols);
        let avail = (max.saturating_sub(text_x)).saturating_sub(1) as usize;
        let mut start = chars.len();
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
        let end_x = crate::chatwidth::place_row(text_x, max, styled, |x, c, (fg, bold)| {
            out.push(cell(x, top, c, fg, bold))
        });
        if end_x < max {
            out.push(cell(end_x, top, '\u{258f}', accent, false)); // ▏
        }
        return;
    }

    let legend = if p.editing.is_some() {
        "edit".to_string()
    } else if let Some(h) = &hit {
        format!("due {}", duedate::label_naive(h.due, h.has_time, now))
    } else if let Some(f) = &p.filter {
        format!("@{f}")
    } else {
        "new".to_string()
    };
    let legend_fg = if hit.is_some() { accent } else { t.legend_off };
    for mut c in
        crate::boxdraw::titled_card(cols, ch, &legend, t.border_normal, legend_fg, t.page_bg)
    {
        c.row += top;
        out.push(c);
    }

    let (text_x, max) = (4u16, cols - 1);
    out.push(cell(2, top + 1, '\u{276f}', accent, true)); // ❯
    if p.input.is_empty() {
        let hint = "type a todo";
        let styled = hint.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(text_x, max, styled, |x, c, ()| {
            out.push(cell(x, top + 1, c, t.text_muted, false))
        });
        return;
    }

    // Show the last `ch - 2` wrapped lines (all of them until the cap bites).
    let lines = input_lines(p, cols);
    let skip = lines.len().saturating_sub((ch - 2) as usize);
    let (mut end_x, mut end_row) = (text_x, top + 1);
    for (vi, &(s, e)) in lines[skip..].iter().enumerate() {
        let r = top + 1 + vi as u16;
        let styled = chars[s..e]
            .iter()
            .enumerate()
            .map(|(j, &c)| (c, style(s + j)));
        end_x = crate::chatwidth::place_row(text_x, max, styled, |x, c, (fg, bold)| {
            out.push(cell(x, r, c, fg, bold))
        });
        end_row = r;
    }
    // The wrap budget leaves this column free ([`input_lines`]).
    out.push(cell(end_x, end_row, '\u{258f}', accent, false)); // ▏
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

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
