//! Assembling panes into `PaneScene`s for `renderer.frame`. Each pane is a
//! fieldset card (see [`crate::panecard`]): the content and its rounded border
//! ride separate text buffers so the border never shifts the content.
use crew_render::PaneScene;

use crate::pane::{Pane, PaneContent};
use crate::panecard::{pane_card, Bar};

/// Build the `PaneScene`s for one frame. Each pane yields **two** scenes — the
/// content, inset by one cell on every side, and the border card around it —
/// kept in separate text buffers so the box-drawing border glyphs never share a
/// line with (and so never shift) the content. `broadcast` marks terminal panes
/// receiving synchronized input; `find` is the active `/find` term, highlighted
/// in the focused pane while scrolled back.
pub fn build_scenes(
    panes: &[Pane],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
    cw: f32,
    ch: f32,
) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    let mut scenes = Vec::with_capacity(panes.len() * 2);
    for (i, p) in panes.iter().enumerate() {
        let foc = focused == Some(i);
        let mut cells = p.cells(foc);
        let is_term = matches!(&p.content, PaneContent::Terminal(_));
        let scroll = match &p.content {
            PaneContent::Terminal(t) => t.pty.display_offset(),
            _ => 0,
        };
        // Tint http(s) URLs blue so they read as clickable (Cmd+click opens).
        if is_term {
            crate::linkhl::colorize(&mut cells, p.grid.cols, p.grid.rows);
        }
        // Wash search matches in the focused terminal while viewing a /find
        // result (scrolled back); it self-clears on return to the bottom.
        if foc && is_term && scroll > 0 {
            if let Some(term) = find {
                crate::findhl::highlight(&mut cells, term, p.grid.cols, p.grid.rows);
            }
        }
        let r = p.rect;
        // Content: its own buffer, inset one cell past the top-left border so it
        // starts exactly on the grid (no leading border glyph to push it).
        scenes.push(PaneScene {
            cells,
            x: r.x + cw,
            y: r.y + ch,
            w: (r.w - 2.0 * cw).max(0.0),
            h: (r.h - 2.0 * ch).max(0.0),
            focused: foc,
            bordered: false,
            overlay: false,
        });
        // Border card: the rounded frame + legend + status, drawn over the rect.
        let title = p.title_text();
        scenes.push(PaneScene {
            cells: pane_card(
                p.grid.cols,
                p.grid.rows,
                &Bar {
                    index: multi.then_some(i + 1),
                    title: &title,
                    focused: foc,
                    scroll,
                    activity: p.activity && !foc,
                    bell: p.bell && !foc,
                    broadcast: broadcast && is_term,
                },
            ),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            focused: foc,
            bordered: false,
            overlay: false,
        });
    }
    scenes
}
