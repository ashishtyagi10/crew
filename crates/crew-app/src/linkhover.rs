//! The pointer knows a link when it is over one.
//!
//! URLs and file references are already drawn as links — tinted, and ruled
//! underneath, so they read as clickable without depending on hue. What the
//! pointer did over one was nothing at all: the same I-beam it wears over
//! every other character in the pane, on text that is one modifier away from
//! opening a browser or a viewer. A click target with no affordance is a
//! secret, which is the argument the toast stack's hover already makes and
//! the border buttons' before it.
//!
//! Over a marked run the pointer becomes a hand and the run goes **bold**.
//! Bold rather than a colour change: the run is already carrying the link
//! colour to say what it is, and a hover that changed that hue would be
//! saying the second thing in the same channel as the first.
//!
//! ## What counts as a link
//!
//! Exactly what is drawn as one — the URL spans [`crate::linkhl`] tints and
//! the file references [`crate::pathhl`] rules. Deliberately not "any token
//! that happens to name a file on disk", which is what Cmd+click will *also*
//! open: answering that means a filesystem check, and this runs on every
//! pointer move. The hover promises what the drawing already promised, and
//! the click is free to find more.
//!
//! Published through an atomic and read while the pane's cells are built, for
//! the same reason [`crate::panehover`] is: hover is a property of the frame,
//! and the scene-building call chain is already at clippy's argument limit.
use std::sync::atomic::{AtomicU64, Ordering};

use crew_render::CellView;
use crew_term::TermModel;

/// `(pane + 1) << 48 | row << 32 | start << 16 | end`, or 0 for "the pointer
/// is over no link". Every field is bounded well under 16 bits by the grid
/// itself; a pane index past 255 or a column past 65535 simply publishes
/// nothing rather than aliasing onto another pane's run.
static HOVER: AtomicU64 = AtomicU64::new(0);

fn encode(h: Option<(usize, u16, usize, usize)>) -> u64 {
    let Some((pane, row, a, b)) = h else { return 0 };
    // One-based, so pane 0 is a pane and not an absence. `checked_add` rather
    // than `+ 1`: this is fed a `usize` from the pane list, and a saturating
    // index would panic here in debug before it could be rejected below.
    let (Some(pane), Ok(a), Ok(b)) = (pane.checked_add(1), u16::try_from(a), u16::try_from(b))
    else {
        return 0;
    };
    let Ok(pane) = u64::try_from(pane) else {
        return 0;
    };
    if pane > 0xFFFF {
        return 0;
    }
    pane << 48 | u64::from(row) << 32 | u64::from(a) << 16 | u64::from(b)
}

/// Serialises the tests that publish to the process-global hover. Without it
/// they race under the parallel runner and each reads whatever the last one
/// happened to store.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Publish this frame's hovered link run. Returns `true` when it differs from
/// what was already published — the signal to schedule a redraw.
pub(crate) fn publish(h: Option<(usize, u16, usize, usize)>) -> bool {
    let v = encode(h);
    HOVER.swap(v, Ordering::Relaxed) != v
}

/// The hovered run in pane `pane`, as `(row, start, end)`. Panes other than
/// the hovered one get `None`, so exactly one run on the canvas ever lights.
fn for_pane(pane: usize) -> Option<(u16, usize, usize)> {
    let v = HOVER.load(Ordering::Relaxed);
    let idx = (v >> 48) as usize;
    if idx == 0 || idx - 1 != pane {
        return None;
    }
    Some((
        ((v >> 32) & 0xFFFF) as u16,
        ((v >> 16) & 0xFFFF) as usize,
        (v & 0xFFFF) as usize,
    ))
}

/// Whether the pointer is on a link right now — what the pointer shape asks.
pub(crate) fn any() -> bool {
    HOVER.load(Ordering::Relaxed) != 0
}

/// The half-open column range of the drawn link under `col` in `line`, or
/// `None`. URLs first: a URL containing something that looks like a path
/// (`https://host/a/b.rs`) is a URL, and the two matchers overlap there.
pub(crate) fn span_at(line: &str, col: usize) -> Option<(usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let covers = |(a, b): &(usize, usize)| (*a..*b).contains(&col);
    crate::openurl::url_spans(&chars)
        .into_iter()
        .find(covers)
        .or_else(|| crate::pathhl::path_spans(&chars).into_iter().find(covers))
}

/// Embolden the hovered run in `pane`'s cells. A no-op on every pane but the
/// one under the pointer, and on every row but the one it is on.
pub(crate) fn mark(cells: &mut [CellView], pane: usize) {
    let Some((row, a, b)) = for_pane(pane) else {
        return;
    };
    for c in cells
        .iter_mut()
        .filter(|c| c.row == row && (a..b).contains(&(c.col as usize)))
    {
        c.bold = true;
    }
}

impl crate::app::CrewApp {
    /// The link run under the pointer, as `(pane, row, start, end)`.
    ///
    /// Row text is reconstructed for the ONE row under the pointer, at
    /// pointer-move time — never pre-scanned during layout. That is the same
    /// discipline `clickopen` follows for the click this hover is announcing,
    /// and it is what makes a per-move answer affordable.
    pub(crate) fn link_under_cursor(&self) -> Option<(usize, u16, usize, usize)> {
        let i = self.pane_at_cursor()?;
        let (row, col) = self.cursor_rowcol(i)?;
        let (row, col) = (u16::try_from(row).ok()?, usize::try_from(col).ok()?);
        let pane = self.panes.get(i)?;
        let line = match &pane.content {
            crate::pane::PaneContent::Terminal(t) => {
                crate::dump::grid_row(&t.pty.cells(false), row, pane.grid.cols)
            }
            crate::pane::PaneContent::Chat(c) => {
                crate::chatview::row_text_at(c, pane.grid.cols, pane.grid.rows, row)?
            }
            // Every other pane kind draws its own links (the viewer's
            // markdown rung carries real `link` cells, resolved on click) or
            // has none at all.
            _ => return None,
        };
        let (a, b) = span_at(&line, col)?;
        Some((i, row, a, b))
    }

    /// Recompute and publish the hovered link. Returns `true` when it moved,
    /// which is the signal to repaint — the run's weight is part of the
    /// frame, so a hover that changed nothing must not cost one.
    pub(crate) fn link_hover_sync(&mut self) -> bool {
        publish(self.link_under_cursor())
    }
}

#[cfg(test)]
#[path = "linkhover_tests.rs"]
mod tests;
