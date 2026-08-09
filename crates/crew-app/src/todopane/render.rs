//! Rendering (and the matching click geometry) for the todo pane: the item
//! list from the top, the `@project` popup and the bordered composer at the
//! bottom. All layout arithmetic lives here so `cells` and [`click_at`] can
//! never disagree about what sits on a row.
use crew_render::CellView;

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

/// Composer rows at this pane height: a 3-row bordered card, or a single
/// bare prompt row on very short panes.
pub(crate) fn composer_h(rows: u16) -> u16 {
    if rows >= 6 {
        3
    } else {
        1
    }
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
pub(crate) fn list_height(p: &TodoPane, rows: u16) -> u16 {
    rows.saturating_sub(composer_h(rows) + popup_h(p, rows) + header_h(p))
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
    if row >= rows.saturating_sub(composer_h(rows)) {
        return Some(TodoClick::Composer);
    }
    let header = header_h(p);
    if row < header {
        return None;
    }
    let di = (row - header) as usize + p.scroll;
    let shown = p
        .visible_len()
        .min(p.scroll + list_height(p, rows) as usize);
    if di >= shown {
        return None;
    }
    Some(if (BOX_COL..BOX_COL + 3).contains(&col) {
        TodoClick::Toggle(di)
    } else if col >= cols.saturating_sub(3) {
        TodoClick::Delete(di)
    } else {
        TodoClick::Select(di)
    })
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
    let lh = list_height(p, rows) as usize;
    for (vi, &idx) in order.iter().skip(p.scroll).take(lh).enumerate() {
        let row = header + vi as u16;
        let selected = p.sel == Some(p.scroll + vi);
        row_cells(&mut out, p, idx, row, cols, selected, now_ms);
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
        let top = rows - composer_h(rows) - ph;
        for mut c in crate::cmdmenu::menu_card("projects", &items, m.sel, cols, ph) {
            c.row += top;
            out.push(c);
        }
    }

    composer_cells(&mut out, p, cols, rows);
    out
}

/// One item row: `› [ ] title … @tag due ✗`, width-aware and clipped.
fn row_cells(
    out: &mut Vec<CellView>,
    p: &TodoPane,
    idx: usize,
    row: u16,
    cols: u16,
    selected: bool,
    now_ms: u64,
) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let it = &p.items[idx];
    let ink = if it.done { t.text_muted } else { t.ink };

    if selected {
        out.push(cell(0, row, '\u{203a}', accent, true)); // ›
    }
    for (i, c) in (if it.done { "[x]" } else { "[ ]" }).chars().enumerate() {
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
        let overdue = !it.done && due <= now_ms;
        let today = duedate::days_from_now(due, now_ms) == Some(0);
        let fg = if it.done {
            t.text_muted
        } else if overdue {
            t.bell
        } else if today {
            t.status_fg
        } else {
            t.text_muted
        };
        right = place_right(out, &lbl, right, row, fg, overdue && !it.done);
    }
    if let Some(tag) = &it.project {
        let chip = format!("@{tag}");
        let fg = if it.done { t.text_muted } else { accent };
        right = place_right(out, &chip, right, row, fg, false);
    }
    // `right` is the next free slot two left of the leftmost right-side
    // text; the title keeps a one-column gap before that text.
    let title_max = right + 1;
    let styled = it.title.chars().map(|c| (c, ()));
    crate::chatwidth::place_row(TITLE_COL, title_max, styled, |x, c, ()| {
        out.push(cell(x, row, c, ink, selected))
    });
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
/// interior row the `❯ input▏` prompt with the date fragment and `@tags`
/// tinted as they're typed.
fn composer_cells(out: &mut Vec<CellView>, p: &TodoPane, cols: u16, rows: u16) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let ch = composer_h(rows);
    let top = rows - ch;
    let now = duedate::now_local();
    let hit = duedate::find(&p.input, now);

    let (x0, max, prompt_row) = if ch == 3 {
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
            crate::boxdraw::titled_card(cols, 3, &legend, t.border_normal, legend_fg, t.page_bg)
        {
            c.row += top;
            out.push(c);
        }
        (2u16, cols - 1, top + 1)
    } else {
        (0u16, cols, top)
    };

    out.push(cell(x0, prompt_row, '\u{276f}', accent, true)); // ❯
    let text_x = x0 + 2;
    if p.input.is_empty() {
        let hint = "type a todo";
        let styled = hint.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(text_x, max, styled, |x, c, ()| {
            out.push(cell(x, prompt_row, c, t.text_muted, false))
        });
        return;
    }

    let chars: Vec<char> = p.input.chars().collect();
    let in_date = |i: usize| hit.as_ref().is_some_and(|h| i >= h.start && i < h.end);
    let tag_spans = tag_spans(&chars);
    let in_tag = |i: usize| tag_spans.iter().any(|&(s, e)| i >= s && i < e);
    // Tail-follow: show the last chars that fit, editing happens at the end.
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
    let styled = chars[start..].iter().enumerate().map(|(j, &c)| {
        let i = start + j;
        let (fg, bold) = if in_date(i) {
            (accent, true)
        } else if in_tag(i) {
            (accent, false)
        } else {
            (t.ink, false)
        };
        (c, (fg, bold))
    });
    let end_x = crate::chatwidth::place_row(text_x, max, styled, |x, c, (fg, bold)| {
        out.push(cell(x, prompt_row, c, fg, bold))
    });
    if end_x < max {
        out.push(cell(end_x, prompt_row, '\u{258f}', accent, false)); // ▏
    }
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
