//! Asking the crew pane what is at a point: the link under the cursor, the
//! text of a row, the code block a click landed in, and where the busy wash
//! sits.
//!
//! Split from [`crate::chatview`] for the line cap, along the line between
//! DRAWING the transcript and interrogating what was drawn.
use crate::chat::ChatPane;
use crate::chatbody::CardCell;
use crew_render::CellView;

/// Wash the CURRENT `chatfind` match's on-screen substring cells with the
/// theme's `find_hl_bg` — the same colour the terminal `/find` wash
/// (`findhl`) and mouse selection use, calibrated for light and dark alike.
/// Only the current match's rows are washed, so it reads as "you are here",
/// not "everywhere". All indexing is `get`-guarded: the transcript may have
/// shifted since the matches were recorded, and a stale index must skip,
/// never panic.
pub(crate) fn find_wash(pane: &ChatPane, cols: u16, rows: u16, cells: &mut [CellView]) {
    let Some(f) = &pane.find else { return };
    let Some(&mi) = f.matches.get(f.sel) else {
        return;
    };
    let top = pane.status_rows(cols, rows);
    if f.query.is_empty() || top == 0 || cols == 0 {
        return;
    }
    let budget = crate::chatplace::msg_rows_budget(pane, cols, rows) as usize;
    let visible = pane.visible_messages();
    let view = crate::chatmsgs::View {
        source: pane.show_source,
        compact: pane.compact_view,
        gap_rows: crate::density::level().card_gap_rows(),
        streaming_from: pane.messages.len(),
    };
    let (lines, spans) = crate::chatmsgs::card_lines_spanned(&visible, cols as usize, 0, view);
    let Some(span) = spans.get(mi) else { return };
    // The drawn window, exactly as `chatplace::window` slices AND places it —
    // a short transcript sits on the bottom of its rows, so its first line is
    // `top_pad` below the header, not on it.
    let start = lines
        .len()
        .saturating_sub(budget)
        .saturating_sub(pane.scroll);
    let end = (start + budget).min(lines.len());
    let first = top + crate::chatplace::top_pad(lines.len(), budget as u16);
    for idx in span.start.max(start)..span.end.min(end) {
        wash_row(cells, first + (idx - start) as u16, &f.query, cols);
    }
}

/// Wash `query` occurrences on one absolute pane row — `findhl::highlight`'s
/// column-grid walk, restricted to a single row.
pub(crate) fn wash_row(cells: &mut [CellView], row: u16, query: &str, cols: u16) {
    let needle: Vec<char> = query.chars().map(|c| c.to_ascii_lowercase()).collect();
    let mut hay = vec![' '; cols as usize];
    for c in cells.iter().filter(|c| c.row == row) {
        if let Some(slot) = hay.get_mut(c.col as usize) {
            *slot = c.c.to_ascii_lowercase();
        }
    }
    if needle.is_empty() || needle.len() > hay.len() {
        return;
    }
    let mut marked = vec![false; hay.len()];
    for s in 0..=hay.len() - needle.len() {
        if hay[s..s + needle.len()] == needle[..] {
            marked[s..s + needle.len()]
                .iter_mut()
                .for_each(|m| *m = true);
        }
    }
    let hl = crew_theme::theme().find_hl_bg;
    for c in cells.iter_mut().filter(|c| c.row == row) {
        if marked.get(c.col as usize).copied().unwrap_or(false) {
            c.bg = hl;
        }
    }
}

/// The URL a markdown link occupies at `(row, col)` in the message body, if
/// any — `clickopen`'s click hit-test. Re-derives `chatplace::placed_lines` with
/// the same `cols`/`rows` geometry `cells` renders the message area at, so a
/// click can never resolve against stale layout. `col` is a DISPLAY column
/// (what the click's `CellView` carries), so the cell is found via
/// `chatplace::cell_at_col`, which walks the line with the same char-width
/// accounting `line_cells` renders with — not raw `Vec` indexing, which
/// drifts from display columns once a wide (CJK/emoji) or zero-width glyph
/// appears earlier on the line.
pub(crate) fn link_at(pane: &ChatPane, cols: u16, rows: u16, row: u16, col: u16) -> Option<String> {
    crate::chatplace::placed_lines(pane, cols, rows)
        .into_iter()
        .find(|(r, _)| *r == row)
        .and_then(|(_, line)| crate::chatplace::cell_at_col(&line, col).cloned())
        .and_then(|cell: CardCell| cell.link)
        .map(|l| l.to_string())
}

/// The plain text (no styling) of the message-body row at `row` — the
/// `clickopen` Chat arm's fallback when `link_at` misses: a path an agent
/// wrote is plain text, not a markdown link, so resolving it needs the raw
/// row content to run `token_at`/`open_path_token` against, the same as a
/// terminal pane's Cmd+click. Same `placed_lines` re-derivation as
/// `link_at`/`code_block_at`, so it can never resolve against stale layout,
/// and it only ever reconstructs the ONE clicked row — nothing is scanned
/// ahead of a click.
pub(crate) fn row_text_at(pane: &ChatPane, cols: u16, rows: u16, row: u16) -> Option<String> {
    crate::chatplace::placed_lines(pane, cols, rows)
        .into_iter()
        .find(|(r, _)| *r == row)
        .map(|(_, line)| line.iter().map(|c| c.c).collect())
}

/// The whole fenced code block a click landed in, as text — `None` when the
/// click was not inside one.
///
/// Reading an answer and then USING it are different acts, and the second had
/// no support: a code block could be selected with the mouse like any other
/// text, which is the same amount of work as retyping it for anything longer
/// than a line. Cmd+click already means "act on what is under the cursor"
/// for links; this gives it the other thing worth acting on.
///
/// A block is the run of contiguous rows laid onto the code FIELD — every
/// cell past the indent tinted, which `chatfield` only ever does to a fence.
/// A prose line merely carrying an inline `code` span has tinted cells among
/// untinted ones and is correctly not a block; asking whether ANY cell was
/// tinted made every such line one, and made one next to a fence extend it.
///
/// The run's first row is the language and its last is the field's closing
/// blank, so the copy is the code between them — no language tag, nothing to
/// strip after pasting.
pub(crate) fn code_block_at(pane: &ChatPane, cols: u16, rows: u16, row: u16) -> Option<String> {
    let code_bg = Some(crate::chatink::code_bg());
    let placed = crate::chatplace::placed_lines(pane, cols, rows);
    let is_field = |r: u16| {
        placed
            .iter()
            .find(|(pr, _)| *pr == r)
            .is_some_and(|(_, line)| {
                line.len() > 1 && line[1..].iter().all(|c: &CardCell| c.bg == code_bg)
            })
    };
    if !is_field(row) {
        return None;
    }
    let (mut first, mut last) = (row, row);
    while first > 0 && is_field(first - 1) {
        first -= 1;
    }
    while last + 1 < rows && is_field(last + 1) {
        last += 1;
    }
    // Drop the language row and the closing blank row: they are the field's
    // chrome, not the agent's code.
    let (first, last) = (first + 1, last.checked_sub(1)?);
    if first > last {
        return None;
    }
    let text = |r: u16| -> String {
        placed
            .iter()
            .find(|(pr, _)| *pr == r)
            .map(|(_, line)| line.iter().map(|c| c.c).collect::<String>())
            .unwrap_or_default()
            .trim_end()
            .to_string()
    };
    let body: Vec<String> = (first..=last).map(text).collect();
    // Every code line is indented by one column when placed; strip that back
    // off so what lands on the clipboard is what the agent wrote.
    let indent = body
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    Some(
        body.iter()
            .map(|l| l.chars().skip(indent).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}
