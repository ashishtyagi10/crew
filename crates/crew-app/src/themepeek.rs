//! Wearing a palette before you choose it.
//!
//! The `/theme` picker names twelve palettes and draws a strip of each one's
//! colours beside its name — which tells you what a palette *is* and not what
//! the screen you are looking at will *look like*. Those are different
//! questions, and only one of them can be answered by a swatch: a palette is
//! its ink on its page under its own gradient, with your panes, your code and
//! your agent's output in it.
//!
//! So arrowing onto a palette in the picker puts it on. Leaving the row takes
//! it off again, and the one you had is exactly the one you get back —
//! including when you dismiss the palette without choosing anything, which is
//! the case a preview has to get right or it is a way to lose your theme.
//!
//! ## Only a named palette
//!
//! A rotation mode (`dark`, `auto`) names a *pool*, not a palette, and the
//! one it would land on is a choice crew makes later — previewing "one of
//! these four" by picking one would be showing something the choice does not
//! promise. Those rows leave the current theme alone.
//!
//! ## What a preview deliberately does NOT do
//!
//! It calls [`crew_theme::set_theme`] and nothing else: no config write, no
//! accent re-resolution, no CRT/glass pin clearing, no DECSET-2031 push to
//! the programs in the panes. Those are what *choosing* a theme does
//! (`set_theme_cmd`), and a preview that did them would be a choice with an
//! undo rather than a look. It also does not start a crossfade — a fade per
//! arrow key would lag a whole step behind the selection.
use std::sync::Mutex;

use crew_theme::{Selection, ThemeId};

use crate::suggest::MenuItem;

/// The theme in force before the preview started, or `None` when nothing is
/// being previewed. A `Mutex` rather than an atomic because it is written
/// only on the first and last frame of a preview.
static RESTORE: Mutex<Option<ThemeId>> = Mutex::new(None);

/// The palette a menu row would put on, or `None` for every row that is not a
/// named palette in the `/theme` picker.
fn palette_of(item: &MenuItem) -> Option<ThemeId> {
    let value = item.fill.strip_prefix("/theme ")?.trim();
    match crew_theme::parse_selection(value)? {
        Selection::Fixed(id) => Some(id),
        // A pool is not a palette — see the module doc.
        Selection::Mode(_) => None,
    }
}

/// Put on the palette the selected row names, or take off whatever the last
/// call put on. Returns `true` when the screen changed, which is the caller's
/// signal to repaint.
///
/// Called after every key the input bar handles, so "the menu closed", "the
/// selection moved" and "the text no longer names a theme" are all the same
/// case: whatever is selected now, if anything.
pub(crate) fn sync(menu: &[MenuItem], sel: usize) -> bool {
    let want = menu
        .get(sel.min(menu.len().saturating_sub(1)))
        .and_then(palette_of);
    let mut restore = RESTORE.lock().unwrap_or_else(|e| e.into_inner());
    match want {
        Some(id) => {
            if crew_theme::current_id() == id {
                return false;
            }
            // Remember the REAL theme, once: the second arrow key must not
            // record the first preview as the thing to go back to.
            restore.get_or_insert_with(crew_theme::current_id);
            crew_theme::set_theme(id);
            true
        }
        None => match restore.take() {
            Some(id) => {
                crew_theme::set_theme(id);
                true
            }
            None => false,
        },
    }
}

/// Forget a preview WITHOUT undoing it — what a chosen row does, since the
/// command that runs next is about to set the theme for real and a restore
/// on the way there would flash the old one back.
pub(crate) fn accept() {
    *RESTORE.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

#[cfg(test)]
#[path = "themepeek_tests.rs"]
mod tests;
