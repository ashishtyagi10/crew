//! Drawing the Explorer's tree: its rule, then one row per [`Item`] — indent guides down each
//! open folder, a chevron on a folder, the name. Row 0 of the panel is the rule; item `k` of the
//! scrolled list is on row `k + 1`, which is the mapping a click reads back
//! ([`crate::explorerhit`]).
use std::path::Path;

use crew_render::CellView;

use crate::explorer::Item;

/// Rows above the first item: the section's rule.
pub(crate) const TOP: u16 = 1;

/// The column an item at `depth` starts on: one in, two more per level.
fn x(depth: u16) -> u16 {
    1 + 2 * depth
}

/// What a row can be marked for: the item under the pointer, the file last opened from the
/// tree, and the keyboard's cursor row while the tree holds the keys.
#[derive(Clone, Copy, Default)]
pub(crate) struct Marks<'a> {
    pub hover: Option<usize>,
    pub picked: Option<&'a Path>,
    pub cursor: Option<usize>,
    /// What git says of the tree's files, when the root is in a repository.
    pub git: Option<&'a crate::gitfiles::Files>,
    /// The name being typed to find, and whether it matched ([`crate::explorerfind`]).
    pub find: Option<(&'a str, bool)>,
}

/// The tree's cells for a panel `cols` × `rows`, the list scrolled by `scroll`.
pub(crate) fn cells(
    items: &[Item],
    scroll: usize,
    marks: Marks,
    (cols, rows): (u16, u16),
) -> Vec<CellView> {
    let Marks {
        hover,
        picked,
        cursor,
        git,
        find,
    } = marks;
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    // The prefix in the rule, muted when nothing starts with it.
    let key = find.map_or(String::new(), |(p, _)| crate::explorerfind::legend(p, cols));
    let key_fg = match find {
        Some((_, true)) => t.ink,
        // No match in the danger ink: muted read as ink on the light themes (porcelain shot).
        _ => crew_theme::readable::danger(t),
    };
    let (border, bg) = (t.border_normal, t.page_bg);
    let mut out =
        crate::boxdraw::section_header_key("EXPLORER", &key, cols, border, accent, key_fg, bg);
    let shown = usize::from(rows.saturating_sub(TOP));
    for (k, item) in items.iter().enumerate().skip(scroll).take(shown) {
        let row = TOP + (k - scroll) as u16;
        // A guide under every open folder this item sits inside — the root's excepted, as VS
        // Code draws none for the workspace itself.
        for d in 1..item.depth {
            put(&mut out, x(d), row, cols, "\u{2502}", t.border_normal);
        }
        let lit = hover == Some(k);
        // Folders in the accent's readable ink, files in ink; the file last opened, and the row
        // under the pointer, in the accent itself — "… n more" too, which a click opens in Far.
        let said = git.and_then(|g| crate::explorergit::said(g, item));
        let fg = match () {
            _ if item.more.is_some() && !lit => t.text_muted,
            _ if lit || picked == Some(item.path.as_path()) => accent,
            _ if said.is_some() => crate::explorergit::ink(said),
            _ if item.dir && item.depth > 0 => crate::accentink::accent_ink(),
            _ => t.ink,
        };
        let at = x(item.depth);
        if item.dir && item.more.is_none() {
            let chevron = if item.open { "\u{25be}" } else { "\u{25b8}" };
            put(&mut out, at, row, cols, chevron, t.text_muted);
        }
        let name = shown_name(item);
        // A marked row keeps its last two columns for the mark.
        let end = if said.is_some() {
            cols.saturating_sub(2)
        } else {
            cols
        };
        put(&mut out, at + 2, row, end, &name, fg);
        if let Some(s) = said {
            let letter = crate::explorergit::letter(s);
            put(
                &mut out,
                cols - 1,
                row,
                cols,
                letter,
                crate::explorergit::ink(said),
            );
        }
        if cursor == Some(k) {
            bar(&mut out, row, cols);
        }
    }
    out
}

/// An item's name as its row writes it: the root as VS Code's workspace heading, in capitals.
pub(crate) fn shown_name(item: &Item) -> String {
    match item.depth {
        0 => item.name.to_uppercase(),
        _ => item.name.clone(),
    }
}

/// The keyboard's row as the far pane draws its cursor: the whole row a bar of the accent,
/// deepened until white reads on it, the words on it bold white. The Inbox and Far lists draw
/// their keyboard cursor with it too (`navlistkeys`).
pub(crate) fn bar(out: &mut Vec<CellView>, row: u16, cols: u16) {
    let (ink, fill) = crate::segment::inked(crate::palette::accent());
    for col in 0..cols {
        if !out.iter().any(|c| c.row == row && c.col == col) {
            put(out, col, row, cols, " ", ink);
        }
    }
    for c in out.iter_mut().filter(|c| c.row == row) {
        (c.fg, c.bg, c.bold) = (ink, fill, true);
    }
}

/// Write `s` at `col` on `row`, clipped to the panel.
fn put(out: &mut Vec<CellView>, col: u16, row: u16, cols: u16, s: &str, fg: (u8, u8, u8)) {
    if col >= cols {
        crate::navcut::note(row, s);
        return;
    }
    crate::navtext::put_at(out, s, col, row, cols, fg);
}

#[cfg(test)]
#[path = "explorerdraw_tests.rs"]
mod tests;
