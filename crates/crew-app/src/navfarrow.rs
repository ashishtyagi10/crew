//! One folder row of the nav's Far section (`navfar`), and its `×`.
//!
//! The list only ever grew: a folder deleted, or one gone to once and never again, stayed
//! until twenty newer ones pushed it off. Now a hovered row shows a `×` in its last column
//! that forgets the folder (`farrecent::forget`) — Delete or Backspace does the same on the
//! keyboard's cursor (`navlistkeys`). A folder no longer on disk is drawn muted and struck
//! (`fargone` says which, on a throttle), and the folder the open Far pane is in wears its
//! gutter `▸` in the accent. The draw and the click read ONE column for the `×` ([`x_col`]).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crew_render::CellView;

use crate::app::CrewApp;
use crate::navfar::name_and_parent;
use crate::pane::PaneContent;

/// The `×` column of a panel `cols` wide: its last drawn one (`put_at` stops before `cols - 1`).
pub(crate) fn x_col(cols: u16) -> u16 {
    cols.saturating_sub(2)
}

/// What marks a row beyond its folder: the pointer on it (`lit`) and on its `×` (`on_x`),
/// the open Far pane in it (`here`), the folder gone from disk (`gone`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RowLook {
    pub lit: bool,
    pub on_x: bool,
    pub here: bool,
    pub gone: bool,
}

/// The marks the whole list is drawn with: the hovered item (an index into the list), whether
/// the pointer is on its `×`, the open Far pane's folder, and the folders gone.
#[derive(Clone, Debug, Default)]
pub(crate) struct Marks {
    pub hover: Option<usize>,
    pub on_x: bool,
    pub here: Option<PathBuf>,
    pub gone: Vec<PathBuf>,
}

impl Marks {
    /// How item `k`, folder `dir`, is drawn.
    pub(crate) fn look(&self, k: usize, dir: &Path) -> RowLook {
        let lit = self.hover == Some(k);
        RowLook {
            lit,
            on_x: lit && self.on_x,
            here: self.here.as_deref() == Some(dir),
            gone: self.gone.iter().any(|g| g == dir),
        }
    }
}

/// Folder `dir` on content row `row` of a panel `cols` wide: `▸ name  ~/parent`, the `×` on
/// the hovered row. The text stops a column short of the `×`, hovered or not, so nothing
/// shifts as the pointer comes and goes.
pub(crate) fn draw(out: &mut Vec<CellView>, dir: &Path, row: u16, cols: u16, look: RowLook) {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let ind = crate::navtext::INDENT;
    let max = x_col(cols).saturating_sub(1);
    let (name, at) = name_and_parent(dir);
    let from = out.len();
    let fg = match look {
        RowLook { gone: true, .. } => t.text_muted,
        RowLook { lit: true, .. } => accent,
        _ => t.ink,
    };
    crate::navtext::put_at(out, &name, ind, row, max, fg);
    for c in &mut out[from..] {
        c.bold = true;
        c.deco.strike = look.gone;
    }
    let gutter = if look.here { accent } else { t.text_muted };
    crate::navtext::put_at(out, "\u{25b8}", 1, row, ind, gutter);
    // Where it is, after the name, when the row has room for a few letters of it.
    let col = ind + crate::chatwidth::str_w(&name) as u16 + 2;
    if col + 4 < max {
        crate::navtext::put_at(out, &at, col, row, max, t.text_muted);
    } else if !at.is_empty() {
        crate::navcut::note(row, &at);
    }
    if look.lit {
        let fg = if look.on_x { accent } else { t.text_muted };
        let x = x_col(cols);
        crate::navtext::put_at(out, "\u{00d7}", x, row, x + 1, fg);
    }
}

impl CrewApp {
    /// The recent folder whose `×` is under the pointer, as an index into `farrecent::list`.
    pub(crate) fn far_x_at_cursor(&self) -> Option<usize> {
        let k = self.far_item_at_cursor()?;
        let (panel, _, _) = self.nav_hit_geometry()?;
        let (cw, ch, ..) = self.frame_geometry()?;
        // The panel's interior starts one cell in, past the card's border column; its width
        // is the card's interior less the bar's, as `push_sidebar` lays it out.
        let col = ((self.cursor.0 - panel.x) / cw).floor() as i32 - 1;
        let bar = f32::from(crate::navopen::BAR_TAKES) * cw;
        let (card, _) = crate::layout::card_inner_cells(panel.w + bar, panel.h, cw, ch);
        (u16::try_from(col).ok()? == x_col(panel_cols(card))).then_some(k)
    }

    /// A press on a row's `×`: the folder is forgotten, not opened. `true` when it was the `×`'s.
    pub(crate) fn far_forget_press(&mut self) -> bool {
        let Some(k) = self.far_x_at_cursor() else {
            return false;
        };
        self.far_forget_item(k);
        self.redraw();
        true
    }

    /// Forget recent folder `k` (an index into `farrecent::list`) — the `×` and Delete.
    pub(crate) fn far_forget_item(&mut self, k: usize) {
        if let Some(d) = crate::farrecent::list().get(k).cloned() {
            crate::farrecent::forget(&d);
            self.set_status(format!("far: forgot {}", crate::disktile::short_path(&d)));
        }
    }

    /// The marks the Far list is drawn with now.
    pub(crate) fn far_marks(&self) -> Marks {
        let here = self.far_pane().and_then(|i| match &self.panes[i].content {
            PaneContent::Far(f) => f.active_local_dir(),
            _ => None,
        });
        Marks {
            hover: self.far_item_at_cursor(),
            on_x: self.far_x_at_cursor().is_some(),
            here,
            gone: crate::fargone::gone(),
        }
    }
}

/// The nav panel's columns in a card `card_cols` wide: the card's interior less the activity
/// bar's. The draw (`navopen`) and the `×`'s hit read this one sum, so the `×` is hit where drawn.
pub(crate) fn panel_cols(card_cols: u16) -> u16 {
    card_cols.saturating_sub(crate::navopen::BAR_TAKES)
}

/// The `×` under the pointer last drawn, encoded (0: none).
static X_HOVER: AtomicUsize = AtomicUsize::new(0);

/// Publish this move's `×` hover — the row's hover alone does not change as the pointer slides
/// onto its `×`, and the `×` lights then. `true` when it changed.
pub(crate) fn publish_x(k: Option<usize>) -> bool {
    let code = k.map_or(0, |k| k + 1);
    X_HOVER.swap(code, Ordering::Relaxed) != code
}

#[cfg(test)]
#[path = "navfarrow_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "navfarrowshot_tests.rs"]
mod shot_tests;
