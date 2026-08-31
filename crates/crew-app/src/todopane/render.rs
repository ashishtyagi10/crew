//! Rendering (and the matching click geometry) for the todo pane: the item
//! list from the top, the `@project` popup and the bordered composer at the
//! bottom. All layout arithmetic lives here so `cells` and [`click_at`] can
//! never disagree about what sits on a row.
pub(crate) use super::fitline::*;
use crew_render::CellView;

use super::{composer, duedate, gutter, TodoPane};

/// Column where the `[ ]` checkbox starts; the title follows two past it.
pub(crate) const BOX_COL: u16 = 2;
pub(crate) const TITLE_COL: u16 = 6;

pub(super) fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bold: bool) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg: crew_theme::theme().page_bg,
        bold,
        italic: false,
        ..Default::default()
    }
}

/// Widest the list is laid out, however wide the pane is.
///
/// A row puts its title on the left and its `@project` and due label on the
/// right, so on a full-window pane the due date sat ninety columns from the
/// task it belonged to with nothing in between — the same way the command
/// palette's chords did before they were given a measure. A list has a
/// measure; past it, it is two lists that happen to share a row.
///
/// Applied at every public entry rather than once at the top of [`cells`]:
/// the scroll math (`list_height`, `row_h`) and the click hit-test read the
/// same widths, and a wrapped title's HEIGHT depends on the width it wrapped
/// at, so a capped draw over an uncapped measurement is a list whose rows are
/// not where it thinks they are. Idempotent, so nesting these calls is safe.
pub(crate) fn content(cols: u16) -> u16 {
    cols.min(MAX_LIST_W)
}

/// The header row's done button: the visible, clickable way to reach ticked
/// items. `h` on the list has always done this, but a key that only works
/// once the list has focus is not an affordance — and on an all-done pane
/// there is nothing to focus at all. `None` when there is nothing ticked to
/// show (no button for an empty promise) or inside the history view, which
/// is already done-only.
pub(crate) fn done_chip(p: &TodoPane) -> Option<String> {
    if p.done_view {
        return None;
    }
    let n = super::item::done_count(&p.items, p.filter.as_deref());
    (n > 0).then(|| {
        if p.show_done {
            "[hide done]".to_string()
        } else {
            format!("[show {n} done]")
        }
    })
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
    /// The header row's `[show N done]` / `[hide done]` button.
    ShowDone,
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
    let cols = content(cols);
    if row >= rows.saturating_sub(composer::height(p, cols, rows)) {
        return Some(TodoClick::Composer);
    }
    let header = header_h(p, cols);
    if row == 0 && header > 0 {
        // The only live target on the header row; the rest of it is text.
        return done_chip_zone(p, cols)
            .filter(|&(start, end)| (start..end).contains(&col))
            .map(|_| TodoClick::ShowDone);
    }
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
        let head = u16::from(starts_day_group(&p.items, p.done_view, &order, di));
        let h = item_h(&p.items[idx], cols, now_ms, p.done_view) + head;
        if row < top + h {
            // A day header is not a row of the item it rides on.
            if head == 1 && row == top {
                return None;
            }
            let first = top + head;
            // The checkbox rides the title's first row; the ✗ rides whichever
            // row the chips are on — the same row on a wide pane, the item's
            // last one where the row stacked. Everything else just selects.
            let it = &p.items[idx];
            let del_row = if stacked(it, cols, now_ms, p.done_view) {
                top + h - 1
            } else {
                first
            };
            return Some(if row == first && (BOX_COL..BOX_COL + 3).contains(&col) {
                TodoClick::Toggle(di)
            } else if row == del_row && col >= cols.saturating_sub(3) {
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
    let cols = content(cols);

    if cols < 8 || rows < 2 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let now_ms = crate::chattime::unix_now_ms();
    let order = p.order();
    let mut out = Vec::new();

    if let Some(f) = &p.filter {
        let n = order.len();
        // The `@tag` leads in its own color; the ` · 3 items` tail stays muted.
        let head = format!("@{f}");
        let tag_fg = crew_theme::tag_color(f, t);
        let x = crate::chatwidth::place_row(
            BOX_COL,
            cols,
            head.chars().map(|c| (c, ())),
            |x, c, ()| out.push(cell(x, 0, c, tag_fg, false)),
        );
        let tail = format!(" · {n} item{}", if n == 1 { "" } else { "s" });
        crate::chatwidth::place_row(x, cols, tail.chars().map(|c| (c, ())), |x, c, ()| {
            out.push(cell(x, 0, c, t.text_muted, false))
        });
    }
    // The done button rides the same header row, right-aligned like the pane
    // card's own [-] and [x]. Accent, because it is the one thing on that row
    // you can click.
    if let (Some(chip), Some((start, _))) = (done_chip(p), done_chip_zone(p, cols)) {
        let styled = chip.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(start, cols, styled, |x, c, ()| {
            out.push(cell(x, 0, c, crate::palette::accent(), false))
        });
    }

    let header = header_h(p, cols);
    let lh = list_height(p, cols, rows) as usize;
    let bottom = header + lh as u16;
    let today = duedate::now_local().date();
    let mut row = header;
    for (di, &idx) in order.iter().enumerate().skip(p.scroll) {
        if row >= bottom {
            break;
        }
        if starts_day_group(&p.items, p.done_view, &order, di) {
            let label = match done_day(&p.items[idx]) {
                Some(d) => duedate::day_label_naive(d, today),
                None => "earlier".to_string(),
            };
            let styled = label.chars().map(|c| (c, ()));
            crate::chatwidth::place_row(BOX_COL, cols, styled, |x, c, ()| {
                out.push(cell(x, row, c, t.text_muted, true))
            });
            row += 1;
            if row >= bottom {
                break;
            }
        }
        let selected = p.sel == Some(di);
        row_cells(&mut out, p, idx, (row, cols, bottom), selected, now_ms);
        row += item_h(&p.items[idx], cols, now_ms, p.done_view);
    }
    if order.is_empty() && lh >= 2 {
        // An all-done list must not read as a fresh one. With every item
        // ticked there are no rows left, so `H` (a list key) can't even be
        // reached from here — Tab has nothing to select. The way in from an
        // empty pane is the command, so that is what the hint names.
        let done = super::item::done_count(&p.items, p.filter.as_deref());
        let all_done = format!("all done · {done} in the history");
        let none_here = p.filter.as_deref().map(|f| format!("nothing done in @{f}"));
        let hints: [&str; 2] = if p.done_view {
            [
                none_here.as_deref().unwrap_or("nothing done yet"),
                "tick an item on the list — it lands here",
            ]
        } else if done > 0 {
            [&all_done, "/todo done opens the log"]
        } else {
            [
                "no todos",
                "type one below — try: pay rent tomorrow 5pm @home",
            ]
        };
        for (i, hint) in hints.iter().enumerate() {
            let row = header + (lh as u16 / 2).saturating_sub(1) + i as u16;
            let styled = hint.chars().map(|c| (c, ()));
            crate::chatwidth::place_row(BOX_COL, cols, styled, |x, c, ()| {
                out.push(cell(x, row, c, t.text_muted, false))
            });
        }
    }

    // The list's own scroll reading, over the rows the list was given.
    let row_of = |di: usize| row_h(&p.items, p.done_view, &order, di, cols, now_ms);
    let total: u16 = (0..order.len()).map(row_of).sum();
    let above: u16 = (0..p.scroll.min(order.len())).map(row_of).sum();
    out.extend(gutter::cells(above, total, header, lh as u16, cols - 1));

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
                color: Some(crew_theme::tag_color(tag, t)),
                ..Default::default()
            })
            .collect();
        let top = rows - composer::height(p, cols, rows) - ph;
        for mut c in crate::cmdmenu::menu_card("projects", &items, m.sel, cols, ph) {
            c.row += top;
            out.push(c);
        }
    }

    composer::cells(&mut out, p, cols, rows);
    out
}

/// One item: `› [ ] title … @tag due ✗` on its first row, the title
/// wrapping onto full-width continuation rows below ([`title_lines`]);
/// rows at or past `bottom` are clipped.
/// `(row, cols, bottom)`: where this item's first row sits, how wide it may
/// draw, and the row it must stop before.
type RowBox = (u16, u16, u16);

pub(crate) fn row_cells(
    out: &mut Vec<CellView>,
    p: &TodoPane,
    idx: usize,
    (row, cols, bottom): RowBox,
    selected: bool,
    now_ms: u64,
) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let it = &p.items[idx];
    // Done rows (visible under the `h` toggle) sink into the muted tone.
    let ink = if it.done { t.text_muted } else { t.ink };

    if selected {
        out.push(cell(0, row, '\u{203a}', accent, true)); // ›
    }
    let boxes = if it.done { "[x]" } else { "[ ]" };
    for (i, c) in boxes.chars().enumerate() {
        out.push(cell(BOX_COL + i as u16, row, c, ink, selected));
    }

    // The title's lines, then the right side beside the first of them — or,
    // on a pane too narrow to share a line, on a row of its own below.
    let lines = title_lines(it, cols, now_ms, p.done_view);
    let stack = stacked(it, cols, now_ms, p.done_view);
    let chip_row = row + if stack { lines.len() as u16 } else { 0 };

    // Right side, laid right-to-left: ✗, due, @tag.
    let del_col = del_col(cols);
    let mut right = del_col.saturating_sub(2);
    if chip_row < bottom {
        out.push(cell(
            del_col,
            chip_row,
            '\u{2717}', // ✗
            if selected { t.ink } else { t.text_muted },
            false,
        ));
        if p.done_view {
            // The history shows WHEN it was ticked; the day is the header's.
            if let Some(d) = it.done_ms.and_then(duedate::from_epoch_ms) {
                use chrono::Timelike;
                let lbl = format!("{:02}:{:02}", d.time().hour(), d.time().minute());
                right = place_right(out, &lbl, right, chip_row, t.text_muted, false);
            }
        } else if let Some(due) = it.due_ms {
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
            right = place_right(out, &lbl, right, chip_row, fg, overdue);
        }
        if let Some(tag) = &it.project {
            let chip = format!("@{tag}");
            let fg = crew_theme::tag_color(tag, t);
            right = place_right(out, &chip, right, chip_row, fg, false);
        }
    }
    // `right` is the last column the title's first line may use when the
    // chips ride beside it — a two-column gap before their text.
    debug_assert!(
        stack || chip_row >= bottom || right == inline_max(it, cols, now_ms, p.done_view)
    );
    let chars: Vec<char> = it.title.chars().collect();
    for (li, &(s, e)) in lines.iter().enumerate() {
        let r = row + li as u16;
        if r >= bottom {
            break;
        }
        let max = if li == 0 && !stack { right } else { cols - 2 };
        let styled = chars[s..e].iter().map(|&c| (c, ()));
        crate::chatwidth::place_row(TITLE_COL, max, styled, |x, c, ()| {
            out.push(cell(x, r, c, ink, selected))
        });
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
