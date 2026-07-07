//! Precomputed per-width source/preview lines for `MdPane`. `cells`,
//! `link_at`, and `clamp_scrolls` each independently re-ran `md::render`
//! over the whole file and re-wrapped the raw source on every redraw and
//! every click (spec §5 promised precomputed lines). `MdPane::cache_for`
//! rebuilds this only when the column width changes or `MdPane::reload`
//! invalidates it, and all three callers read through it instead.
use std::cell::Ref;

use crate::chatbody::CardLine;
use crate::mdpane::{geometry, MdPane};

/// Right-aligned line-number width (4 digits) plus one separating space.
pub(crate) const GUTTER_W: usize = 5;

/// One `cols`-width's worth of precomputed source/preview lines.
pub(crate) struct MdCache {
    pub(crate) cols: u16,
    pub(crate) wrapped_src: Vec<(usize, Vec<char>)>,
    pub(crate) preview: Vec<CardLine>,
}

impl MdPane {
    /// Rebuilds the wrapped-source/preview-line cache for `cols` if it's
    /// stale (first use, a column-width change, or a `reload` invalidation)
    /// and returns a borrow of it. `cells`, `link_at`, and `clamp_scrolls`
    /// all read through this instead of independently re-parsing the whole
    /// file on every redraw/click.
    pub(crate) fn cache_for(&self, cols: u16) -> Ref<'_, MdCache> {
        {
            let mut cache = self.cache.borrow_mut();
            let stale = !matches!(&*cache, Some(c) if c.cols == cols);
            if stale {
                #[cfg(test)]
                self.rebuilds.set(self.rebuilds.get() + 1);
                let (left_w, _, _, right_w) = geometry(cols);
                let text_w = left_w.saturating_sub(GUTTER_W);
                *cache = Some(MdCache {
                    cols,
                    wrapped_src: wrap_source(&self.source, text_w),
                    preview: preview_lines(&self.source, right_w),
                });
            }
        }
        Ref::map(self.cache.borrow(), |c| {
            c.as_ref().expect("just built above")
        })
    }
}

/// `source` rendered through the shared markdown engine and mapped to card
/// lines exactly like chat bodies (`chatmd::map_lines`), at `right_w`
/// columns — the single place `cache_for` reads the preview half from, so
/// `cells` and `link_at` (both reading the cache it builds) always agree on
/// exactly which line sits where.
///
/// `chatmd::map_lines` prepends an unconditional one-column indent cell to
/// every line (matching the chat card layout it shares code with), so
/// content is wrapped one column narrower than `right_w`, same as
/// `chatbody::body_lines` does for chat cards — otherwise the last column of
/// every width-filling row would be clipped when `line_cells` draws at
/// `right_w` columns.
fn preview_lines(source: &str, right_w: usize) -> Vec<CardLine> {
    let fg = crew_theme::theme().ink;
    let content_w = right_w.saturating_sub(1);
    crate::chatmd::map_lines(crate::md::render(source, content_w), content_w, fg)
}

/// Hard-wraps `source` at `text_w` display columns, tagging every wrapped
/// row with its 1-based source line number (continuation rows share their
/// line's number so callers know whether to reprint the gutter digits).
fn wrap_source(source: &str, text_w: usize) -> Vec<(usize, Vec<char>)> {
    let mut out = Vec::new();
    for (i, line) in source.split('\n').enumerate() {
        let n = i + 1;
        let chars: Vec<char> = line.chars().collect();
        if text_w == 0 || chars.is_empty() {
            out.push((n, Vec::new()));
            continue;
        }
        let mut s = 0;
        while s < chars.len() {
            let e = crate::chatwidth::fit_end(&chars, s, text_w);
            out.push((n, chars[s..e].to_vec()));
            s = e;
        }
    }
    out
}

#[cfg(test)]
#[path = "mdcache_tests.rs"]
mod tests;
