//! Sidebar PANES section: a live list of open panes (index, name/title, a `▸`
//! focus marker, and an activity dot) so the whole grid is visible at a glance —
//! handy when a single pane is zoomed. The crew total rides the section's own
//! rule (`PANES 3`) and busy rows carry a live spinner in the accent color.
//!
//! There used to be a working / waiting / idle breakdown under the header —
//! three rows of chips and counts. It went: the rows below already SAY which
//! pane is which (the spinner, the bell, the unread count), so the tally only
//! restated them a second time, three rows further from the pane it was about,
//! and those three rows are pane rows now.
use crew_render::CellView;

use crate::palette::accent;

/// One row of the PANES list.
pub struct PaneRow {
    pub index: usize,
    pub title: String,
    pub focused: bool,
    pub activity: bool,
    /// Not visible in the content area — minimized into the nav (the pane's
    /// `[-]` border button) or covered while another pane is zoomed: drawn with
    /// a right-aligned `[+]`; clicking the row focuses the pane, which brings
    /// it back on screen.
    pub minimized: bool,
    /// A raised "needs you" marker: `(glyph, visible)`. The glyph names the
    /// event (`!` bell · `⚑` pattern · `✓` command done); `visible` is the
    /// blink phase (false hides the marker mid-pulse, the tint stays). Drawn
    /// in the bell colour, superseding the quiet activity dot.
    pub attention: Option<(char, bool)>,
    /// Doing background work (swarm running, agent chat awaiting, Far op):
    /// the row's dot slot spins while this holds. Attention still wins.
    pub busy: bool,
    /// Lines that arrived since this pane was last read
    /// ([`crate::unread`]) — the same count the pane's own card and its
    /// minimized thumbnail carry. `0` draws nothing.
    pub unread: usize,
    /// The pointer is on this row. The whole row is a click target that
    /// focuses (and restores) the pane, and nothing said so: it lifts its ink
    /// out of the muted grey rather than washing a background behind it,
    /// because the page's contrast headroom is already spent.
    pub hovered: bool,
}

/// Render the PANES section: a `PANES n` rule on row 0, then one row per pane
/// (up to `limit`) directly beneath it.
pub fn pane_cells(panes: &[PaneRow], cols: u16, limit: usize, spin: char) -> Vec<CellView> {
    let t = crew_theme::theme();
    // The crew size rides the rule, the way the LOG's depth and the charts'
    // peaks do — and it is the one number a glance wants.
    let key = panes.len().to_string();
    let mut out = crate::boxdraw::section_header_key(
        "PANES",
        &key,
        cols,
        t.border_normal,
        accent(),
        t.text_muted,
        t.page_bg,
    );
    for (k, p) in panes.iter().take(limit).enumerate() {
        let row = 1 + k as u16;
        let head = format!("{} {}", if p.focused { '▸' } else { ' ' }, p.index);
        let head_fg = if p.focused || p.hovered {
            accent()
        } else {
            t.text_muted
        };
        write(&mut out, &head, 2, row, head_fg, cols - 1, t.page_bg);
        let tstart = 2 + head.chars().count() as u16 + 1;
        let title_fg = if p.focused || p.hovered {
            t.ink
        } else if p.attention.is_some() {
            t.bell
        } else {
            t.text_muted
        };
        // Everything to the right of the title is placed from the edge
        // inward, each item claiming its columns only if what is left still
        // leaves the title something to say. A narrow nav used to place them
        // all unconditionally and let them overprint each other and the
        // title — invisible in a screenshot, since the last writer wins.
        //
        // The order is the priority: the dot slot is reserved by the row
        // itself, then the `[+]` (it is the row's only control), then the
        // count.
        const MIN_TITLE: u16 = 3;
        // `claim(w)` takes `w` columns immediately left of the free edge,
        // with one column of air between it and whatever it sits beside, and
        // returns where to start drawing. `None` when what remains would not
        // leave the title anything to say.
        let mut rx = cols.saturating_sub(2);
        let mut claim = |w: u16| -> Option<u16> {
            let start = rx.checked_sub(w + 1)?;
            (start > tstart + MIN_TITLE).then(|| {
                rx = start;
                start
            })
        };
        let plus = p.minimized.then(|| claim(3)).flatten();
        let count = crate::unread::badge(p.unread)
            .filter(|_| !p.focused)
            .and_then(|n| claim(n.chars().count() as u16).map(|x| (x, n)));
        // Ellipsized, not cut: the row's own markers are placed from the right
        // edge inward and the title takes what is left, so on a narrow nav it
        // is the title that runs short — and a title that stops mid-word looks
        // like a pane that is called that.
        //
        // And one column of air before whatever sits at `rx` — the dot slot,
        // the `[+]`, the count — the same air `claim` keeps between those.
        // Without it a title cut to fit ran straight into its own marker
        // (`cargo wat…12 ●`, `far ~/co…[+]`). A row with nothing at its
        // right keeps the column for its title; a blinking marker counts as
        // there in both phases, or the title would jitter with it.
        let occupied =
            plus.is_some() || count.is_some() || p.attention.is_some() || p.busy || p.activity;
        let room = rx.saturating_sub(tstart + u16::from(occupied));
        let fit = crate::chatwidth::clip_w(&p.title, room as usize);
        write(&mut out, &fit, tstart, row, title_fg, rx, t.page_bg);
        if let Some(x) = plus {
            write(&mut out, "[+]", x, row, accent(), cols, t.page_bg);
        }
        if let Some((x, n)) = count {
            write(&mut out, &n, x, row, t.activity, cols, t.page_bg);
        }
        // The attention marker owns the dot slot while raised; the quiet
        // activity dot returns once the pane has been looked at.
        if let Some((glyph, on)) = p.attention {
            if on {
                write(
                    &mut out,
                    &glyph.to_string(),
                    cols.saturating_sub(2),
                    row,
                    t.bell,
                    cols,
                    t.page_bg,
                );
            }
        } else if p.busy {
            // Live spinner: the busy pane repaints continuously anyway, so the
            // sidebar frame is free — the row visibly *works*.
            write(
                &mut out,
                &spin.to_string(),
                cols.saturating_sub(2),
                row,
                accent(),
                cols,
                t.page_bg,
            );
        } else if p.activity {
            write(
                &mut out,
                "●",
                cols.saturating_sub(2),
                row,
                t.activity,
                cols,
                t.page_bg,
            );
        }
    }
    out
}

/// Write `s` at `(col, row)`, stopping before `max_col`.
fn write(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    max_col: u16,
    bg: (u8, u8, u8),
) {
    // Width-aware: pane titles can carry emoji/CJK (OSC titles) — a wide
    // glyph advances two columns (see `chatwidth`).
    crate::chatwidth::place_row(col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold: false,
            italic: false,
            ..Default::default()
        });
    });
}

#[cfg(test)]
#[path = "panelistgap_tests.rs"]
mod gap_tests;
#[cfg(test)]
#[path = "panelist_tests.rs"]
mod tests;
