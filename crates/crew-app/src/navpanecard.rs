//! A pane's entry in the nav's PANES list as a two-line card, when the
//! panel has the room (the user, 2026-11-24: "we have more space for these
//! panels… a separate entry for each pane").
//!
//! ```text
//! ▎ 7 sentinel     working
//! ▎ ✳ Pondering        12s
//! ```
//!
//! Line one is the pane: its number (Cmd+number), its name, and what state
//! it is in — `working`, `you` (it needs you), `done`, `failed`, `exited`,
//! or nothing when it is idle. Line two is what it is doing: the kind of
//! thing running there and its gist ([`crate::panestate::detail_of`]), with
//! the restore `+` and the unread count at its end. The spine down the left
//! carries the state's colour, so a column of cards reads at a glance; the
//! focused one's spine is wider, and in the accent unless it needs you.
//!
//! Cards are a density, not a mode: [`fits`] decides them from the rows the
//! panel has, and a crew too big for its panel gets the one-line rows back.
//! [`crate::tablines`]'s line map says which, so the draw and every hit test
//! read the same answer.
use crew_render::CellView;

use crate::panelist::PaneRow;
use crate::panestate::EntryState;

/// What a card shows beyond the one-line row.
#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    pub detail: Option<String>,
    pub glyph: char,
    pub state: EntryState,
    /// The card's motion ([`crate::navpanemotion`]); `Default` is still.
    pub fx: crate::navpanemotion::Fx,
}

/// Whether two-line cards fit `rows` content rows (the PANES rule takes one):
/// each pane's second line and a gap between the panes of each group, on top
/// of the one-line list's `compact` lines.
pub(crate) fn fits(rows: u16, compact: usize, panes: usize, groups: usize) -> bool {
    let lines = compact + panes + panes.saturating_sub(groups);
    panes > 0 && lines < usize::from(rows)
}

/// The lines one group of panes takes, in order: a card's two, and a gap
/// between cards; the one-line row alone when `cards` is off.
pub(crate) fn push_entries(
    out: &mut Vec<crate::tabnav::Line>,
    panes: impl Iterator<Item = usize>,
    cards: bool,
) {
    use crate::tabnav::Line;
    for (k, i) in panes.enumerate() {
        if cards && k > 0 {
            out.push(Line::Gap);
        }
        out.push(Line::Pane(i));
        if cards {
            out.push(Line::Detail(i));
        }
    }
}

/// The spine's colour for `state`: the bell's when it needs you or failed,
/// else the accent for the focused card.
fn spine(state: EntryState, focused: bool) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    match state {
        EntryState::Waiting | EntryState::Failed => t.bell,
        _ if focused => crate::palette::accent(),
        EntryState::Working => crate::palette::accent(),
        EntryState::Done => t.activity,
        EntryState::Exited | EntryState::Idle => t.border_normal,
    }
}

/// The state word's colour: the spine's, but never the quiet border —
/// a word is text and keeps a text colour's contrast.
fn word_fg(state: EntryState) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    match state {
        EntryState::Waiting | EntryState::Failed => t.bell,
        EntryState::Working => crate::palette::accent(),
        EntryState::Done => t.activity,
        EntryState::Exited | EntryState::Idle => t.text_muted,
    }
}

/// The card for `r` on rows `row` and `row + 1` of a list `cols` wide,
/// drawn from column 1 as the one-line row's marker is.
pub(crate) fn card_cells(r: &PaneRow, c: &Card, cols: u16, row: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    let mut put = |s: &str, col: u16, row: u16, fg, max: u16, bold: bool| {
        crate::chatwidth::place_row(
            col,
            max,
            s.chars().map(|ch| (ch, fg)),
            |x, ch, marks, fg| {
                out.push(CellView {
                    col: x,
                    row,
                    c: ch,
                    marks,
                    fg,
                    bg: t.page_bg,
                    bold,
                    ..Default::default()
                })
            },
        );
    };
    let now = crate::anim::now_ms();
    let working = c.state == EntryState::Working;
    let edge = crate::navpanemotion::spine_fg(&c.fx, spine(c.state, r.focused), working, now);
    let bar = if r.focused { "\u{258c}" } else { "\u{258e}" };
    put(bar, 1, row, edge, cols, false);
    put(bar, 1, row + 1, edge, cols, false);
    // Line one: number, name, state word from the right edge in.
    let lit = r.focused || r.hovered;
    let num = r.index.to_string();
    let num_fg = if lit {
        crate::palette::accent()
    } else {
        t.text_muted
    };
    put(&num, 3, row, num_fg, cols, false);
    let name_at = 3 + num.chars().count() as u16 + 1;
    let right = cols.saturating_sub(1);
    // The word only where it leaves the name a few columns: on a narrow
    // panel the spine's colour says the state, and the name is the card.
    let word_at = (c.state.word())
        .map(|w| right.saturating_sub(w.chars().count() as u16))
        .filter(|&x| x > name_at + 4);
    let name_end = word_at.map_or(right, |x| x - 1);
    let room = name_end.saturating_sub(name_at) as usize;
    let name = crate::navcut::clip(row, &c.name, room);
    let name_fg = if r.hovered && !r.focused {
        crate::palette::accent()
    } else {
        t.ink
    };
    put(&name, name_at, row, name_fg, name_end, r.focused);
    if let (Some(w), Some(x)) = (c.state.word(), word_at) {
        let fg = crate::navpanemotion::word_fg(&c.fx, word_fg(c.state), now);
        put(w, x, row, fg, right, false);
    }
    // Line two: the kind (under the number, a column of its own), what it is
    // doing, then its time, `+` and the unread count from the edge in.
    let row2 = row + 1;
    put(&c.glyph.to_string(), 3, row2, t.text_muted, cols, false);
    let mut rx = right;
    let mut tail = |s: &str, fg, out_x: &mut Vec<(u16, String, (u8, u8, u8))>| {
        let w = s.chars().count() as u16;
        if rx > 5 + w + 4 {
            rx -= w;
            out_x.push((rx, s.to_string(), fg));
            rx -= 1;
        }
    };
    let mut marks = Vec::new();
    // An agent's elapsed time outlives its words when the line is short:
    // `12s` says more than `Pondering`.
    let (said, time) = match &c.detail {
        Some(d) => crate::panegist::split_time(d),
        None => ("", None),
    };
    if let Some(time) = time {
        tail(time, word_fg(c.state), &mut marks);
    }
    if r.minimized {
        tail("+", crate::palette::accent(), &mut marks);
    }
    // New output is news on a pane at rest; one working is making it, and
    // one waiting has a question that matters more than its count.
    let quiet = matches!(
        c.state,
        EntryState::Idle | EntryState::Done | EntryState::Exited
    );
    if let Some(n) = crate::unread::badge(r.unread).filter(|_| !r.focused && quiet) {
        tail(&n, t.activity, &mut marks);
    }
    let detail_at = 5;
    if !said.is_empty() {
        let room = rx.saturating_sub(detail_at) as usize;
        let d = crate::navcut::clip(row2, said, room);
        let fg = crate::navpanemotion::words_fg(&c.fx, t.text_muted, now);
        put(&d, detail_at, row2, fg, rx, false);
    }
    for (x, s, fg) in marks {
        put(&s, x, row2, fg, cols, false);
    }
    out
}

#[cfg(test)]
#[path = "navpanecard_tests.rs"]
pub(crate) mod tests;
