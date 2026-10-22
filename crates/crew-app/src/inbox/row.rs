//! One thread in the list (goal 2026-10-20-inbox-list-look, phases 1 and 3,
//! made quiet on 2026-10-21: "it should be minimal and look good").
//!
//! Two lines when the pane has room: who wrote, and the time at the right
//! with the star or a paperclip beside it; under them the subject and, in
//! muted ink, the snippet. One line when it has not: the same across. No
//! chips and no rules: the air between threads divides them. An unread
//! thread's names, subject and gutter dot wear a colour of their own
//! ("unread email should appear with different color", 2026-10-22,
//! [`unread_ink`]), and its names, subject and time are bold ("may be bold
//! text"): a family with no Bold face draws bold at its own weight, and the
//! colour still tells. A read one is plain ink with its time receded. The
//! selected row wears the wash from edge to edge, and every colour on it is
//! lifted to read on it.
use crew_render::CellView;

use super::line::Line;
use super::rowfit::{clip, tail, tail_w, Fit, Tail, DOT};
pub(crate) use super::rowfit::{who_w, Ctx, TEXT};
use super::wire::Thread;

type Rgb = (u8, u8, u8);

/// On a two-line row the names have a line of their own.
const WHO_CAP_TWO: usize = 36;
/// Between the subject and the snippet, and before the time.
const AIR: u16 = 2;

/// The thread at `row` (and `row + 1` on two lines).
#[cfg(test)]
pub(crate) fn cells(out: &mut Vec<CellView>, th: &Thread, row: u16, selected: bool, x: &Ctx) {
    let wash = selected.then(|| crew_theme::readable::selection_bg(crew_theme::theme()));
    washed(out, th, row, wash, x);
}

/// [`cells`] on `wash`, edge to edge: the selection's, the lighter one a
/// hovered row wears, or none.
pub(crate) fn washed(out: &mut Vec<CellView>, th: &Thread, row: u16, wash: Option<Rgb>, x: &Ctx) {
    let t = crew_theme::theme();
    let ink = |c: Rgb| match wash {
        Some(w) => crew_theme::readable::against(c, w, crew_theme::contrast::text_floor()),
        None => c,
    };
    let fit = Fit::at(x.cols, x.two);
    // Every row reads names, subject, snippet, bright to quiet; an unread
    // one is the bold one in the accent, with the dot and its time bright.
    let (bright, muted) = (ink(t.ink), ink(t.text_muted));
    let accent = ink(crate::palette::accent());
    let when = if th.unread { bright } else { muted };
    let lead = if th.unread {
        ink(unread_ink(&t))
    } else {
        bright
    };
    let end = x.cols.saturating_sub(1);
    let mut top = Line::new(end);
    if th.unread {
        top.put(DOT, DOT + 1, "\u{25cf}", lead, false);
    }
    let age = match fit.compact {
        true => super::age::short(th.at_ms, x.now),
        false => super::age::said(th.at_ms, x.now),
    };
    let mut right = top.right(TEXT, end, &age, when, th.unread);
    for (on, mark, fg) in [
        (th.starred, "\u{2605}", accent),
        (th.attached && fit.count, clip(), muted),
    ] {
        if on {
            right = top.right(TEXT, right.saturating_sub(1), mark, fg, false);
        }
    }
    let stop = right.saturating_sub(AIR);
    let tails = tail(th, fit);
    let tw = tail_w(&tails);
    let room = match x.two {
        true => usize::from(stop.saturating_sub(TEXT + tw)).min(WHO_CAP_TWO),
        false => usize::from(x.who_w.saturating_sub(tw)),
    };
    let who = super::who::names(th, x.me, room, fit.all);
    let mut after = top.put(TEXT, stop, &who, lead, th.unread);
    for tl in &tails {
        let fg = match tl {
            Tail::Draft => ink(crew_theme::readable::danger(&t)),
            Tail::Count(_) => muted,
        };
        after = top.put(after + 1, stop, tl.text(), fg, false);
    }
    let mut second = x.two.then(|| Line::new(end));
    let (at, stop) = match x.two {
        true => (TEXT, end),
        false => (TEXT + x.who_w + AIR + 1, stop),
    };
    let body = second.as_mut().unwrap_or(&mut top);
    let subject = match th.subject.trim() {
        "" => "(no subject)",
        s => s,
    };
    let after = body.put(at, stop, subject, lead, th.unread);
    if fit.snippet && !th.snippet.is_empty() && after + AIR + 4 < stop {
        body.put(after + AIR, stop, &th.snippet, muted, false);
    }
    top.emit(out, row, wash);
    if let Some(body) = second {
        body.emit(out, row + 1, wash);
    }
}

/// What an unread thread wears: the accent, when it is a colour that stands
/// apart from the ink — the paper themes' accent is a grey, and a tube's is
/// near its ink — else the link blue (a tube's own hue), on the page.
fn unread_ink(t: &crew_theme::Theme) -> Rgb {
    let accent = crate::palette::accent();
    let apart = crew_theme::oklch::from_srgb(accent).c >= 0.05
        && crew_theme::oklch::distance(accent, t.ink) >= 0.1;
    match apart {
        true => accent,
        false => {
            let link = crew_theme::readable::link(t);
            crew_theme::readable::against(link, t.page_bg, crew_theme::contrast::text_floor())
        }
    }
}

#[cfg(test)]
#[path = "row_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rowmarks_tests.rs"]
mod marks_tests;
