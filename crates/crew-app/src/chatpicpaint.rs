//! The pictures a chat transcript paints, read back off its placed rows.
//!
//! `chatimage::box_lines` gives a picture twelve marked blank rows above its
//! caption; `chatglide::place` puts every card row on the pane like any
//! other. This is the pass that turns the marked rows that made it onto the
//! screen back into boxes — anchored at the row the box's first row sits on
//! or would sit on, so a picture half scrolled off the top is the bottom
//! half of the picture and not a squashed whole one — and lays the decoded
//! picture into each, clipped to the message window. Only a picture the
//! cache already holds is drawn (`imgcache::peek`): the layout is the pass
//! that notices an arrival, and it has already given the rows.
use std::path::Path;

use crate::chatbody::CardLine;
use crate::chatlayout::Message;
use crate::chatmsgs::View;
use crew_render::{CellView, Paint};

/// The card view's cells AND the pictures under them — `message_cells` with
/// the paint layer, placed ONCE so the two cannot disagree on a row.
/// `aspect` is the frame's `cell_h / cell_w`.
pub(crate) fn message_art(
    messages: &[&Message],
    cols: u16,
    rows: u16,
    top_row: u16,
    scroll: usize,
    view: View<'_>,
    aspect: f32,
) -> (Vec<CellView>, Vec<Paint>) {
    if cols == 0 || rows == 0 {
        return (Vec::new(), Vec::new());
    }
    let page = crew_theme::theme().page_bg;
    let now = crate::chattime::unix_now_ms();
    let placed = crate::chatglide::place(messages, cols, rows, top_row, scroll, view, now);
    let cells = placed
        .iter()
        .flat_map(|(row, line)| crate::chatplace::line_cells(*row, line, cols, page))
        .collect();
    let clip = (
        0.0,
        f32::from(top_row),
        f32::from(cols),
        f32::from(top_row + rows),
    );
    (cells, paint(&placed, cols, aspect, view.cwd, clip))
}

/// The boxes among `placed`: `(row the box starts on, source)`, one per
/// picture however many of its rows are on screen. The anchor can sit above
/// the window — that is a box cut by the top edge, and the clip takes it.
pub(crate) fn boxes(placed: &[(u16, CardLine)]) -> Vec<(i32, String)> {
    let mut out: Vec<(i32, String)> = Vec::new();
    for (row, line) in placed {
        let Some((i, src)) = crate::chatimage::box_row(line) else {
            continue;
        };
        let anchor = i32::from(*row) - i32::from(i);
        if out.last().is_some_and(|(a, s)| *a == anchor && s == src) {
            continue;
        }
        out.push((anchor, src.to_string()));
    }
    out
}

/// The decoded pictures laid into their boxes, one column in from each
/// card edge like the viewer's, clipped to `clip` (`x0, y0, x1, y1` in cells).
pub(crate) fn paint(
    placed: &[(u16, CardLine)],
    cols: u16,
    aspect: f32,
    cwd: Option<&Path>,
    clip: (f32, f32, f32, f32),
) -> Vec<Paint> {
    let mut out = Vec::new();
    for (anchor, src) in boxes(placed) {
        let Some(path) = crate::imgcache::locate(&src, cwd) else {
            continue;
        };
        let Some(bm) = crate::imgcache::peek(&path) else {
            continue;
        };
        out.extend(crate::viewpane::bitmap::paint_at(
            &bm,
            1.0,
            anchor as f32,
            f32::from(cols).max(2.0) - 2.0,
            f32::from(crate::chatimage::ROWS),
            aspect,
            clip,
        ));
    }
    out
}

#[cfg(test)]
#[path = "chatpicpaint_tests.rs"]
mod tests;
