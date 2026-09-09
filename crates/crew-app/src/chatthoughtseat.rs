//! Where an agent's thought sits in the transcript (see
//! [`crate::chatthought`]): the live block above the agent's streaming card,
//! the folded row above its settled reply, or — with no card of the agent's
//! on screen — a thin card headed by the agent's badge after everything
//! else, exactly as the tool block seats (`chattoolview`). The predicates
//! here are the ones `chatmsgs::card_lines_spanned` draws with AND the ones
//! `chatthoughtfold` resolves a click against, so a row can never be
//! attributed to a block the frame did not put there.
use crate::chatbody::CardLine;
use crate::chatlayout::Message;
use crate::chatmsgs::View;
use crate::chatthoughtview::{block_lines, header, live_lines};

/// Which block a card seats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Seat {
    /// `view.thoughts.live[i]`, above a streaming card.
    Live(usize),
    /// `view.thoughts.settled[i]`, above the reply it is anchored to.
    Settled(usize),
}

/// The block seated above card `m`: the agent's live buffer while the card
/// streams, the block anchored to the card's stamp once it settled.
pub(crate) fn above_of(view: View<'_>, m: &Message, streaming: bool) -> Option<Seat> {
    let key = crate::chatflow::stream_key(&m.sender);
    let t = view.thoughts;
    if streaming {
        return t.live.iter().position(|l| l.agent == key).map(Seat::Live);
    }
    t.settled
        .iter()
        .position(|b| b.agent == key && b.anchor.as_deref() == Some(m.ts.as_str()))
        .map(Seat::Settled)
}

fn agent_of(view: View<'_>, seat: Seat) -> &str {
    match seat {
        Seat::Live(i) => &view.thoughts.live[i].agent,
        Seat::Settled(i) => &view.thoughts.settled[i].agent,
    }
}

fn lines_of(view: View<'_>, seat: Seat, now_ms: u64, cols: usize) -> Vec<CardLine> {
    match seat {
        Seat::Live(i) => live_lines(&view.thoughts.live[i], now_ms, cols),
        Seat::Settled(i) => block_lines(&view.thoughts.settled[i], cols),
    }
}

/// The rows to put above card `m`'s header (before its tool block).
pub(crate) fn above(
    view: View<'_>,
    m: &Message,
    streaming: bool,
    now: u64,
    cols: usize,
) -> Vec<CardLine> {
    above_of(view, m, streaming).map_or(Vec::new(), |s| lines_of(view, s, now, cols))
}

/// [`above`], appended to `out` — the one-line form `card_lines_spanned`
/// calls (that file is at its line cap).
pub(crate) fn push_above(
    view: View<'_>,
    m: &Message,
    streaming: bool,
    now: u64,
    cols: usize,
    out: &mut Vec<CardLine>,
) {
    out.extend(above(view, m, streaming, now, cols));
}

/// The blocks no card seats — a live buffer with no streaming card of its
/// agent's, then every unanchored block — in the order they stand at the end.
pub(crate) fn orphan_seats(view: View<'_>, messages: &[&Message]) -> Vec<Seat> {
    let seated = |s: Seat| {
        messages
            .iter()
            .enumerate()
            .any(|(mi, m)| above_of(view, m, mi >= view.streaming_from) == Some(s))
    };
    let live = (0..view.thoughts.live.len()).map(Seat::Live);
    let settled = (0..view.thoughts.settled.len()).map(Seat::Settled);
    live.chain(settled)
        .filter(|&s| {
            !seated(s)
                && !matches!(s, Seat::Settled(i) if view.thoughts.settled[i].anchor.is_some())
        })
        .collect()
}

/// The unseated blocks as thin cards appended to `out` after the transcript
/// (and after the tool block's own orphans): a card gap when anything
/// precedes, the agent's header, the block's rows.
pub(crate) fn push_orphans(
    view: View<'_>,
    messages: &[&Message],
    now: u64,
    cols: usize,
    out: &mut Vec<CardLine>,
) {
    for seat in orphan_seats(view, messages) {
        if !out.is_empty() {
            out.extend(std::iter::repeat_with(Vec::new).take(view.gap_rows));
        }
        out.push(header(agent_of(view, seat)));
        out.extend(lines_of(view, seat, now, cols));
    }
}

#[cfg(test)]
#[path = "chatthoughtseat_tests.rs"]
mod tests;
