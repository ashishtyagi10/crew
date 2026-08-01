//! The per-reply usage trailer under a settled card: `900 in / 50 out ·
//! $0.012`, from the broker's per-reply `Stats` event. Formatting shares
//! `chathdr::fmt_tokens` and `chatsummary::fmt_cost` with the summary footer,
//! so a reply's numbers and the session totals always read alike.
use crate::chatbody::{plain, CardLine};

/// The trailer text for one reply's `(tok_in, tok_out, cost_microusd)` —
/// `None` when all three are zero: zero-usage stats are real (CLI-backed
/// agents, error hops) and a `0 in / 0 out` line would be noise, not signal.
/// An unpriced model (zero cost, real tokens) keeps its token split and
/// drops the cost segment, mirroring the footer's `cost > 0` gate.
pub(crate) fn trailer_text(tok_in: u64, tok_out: u64, cost_microusd: u64) -> Option<String> {
    if tok_in == 0 && tok_out == 0 && cost_microusd == 0 {
        return None;
    }
    let mut s = format!(
        "{} in / {} out",
        crate::chathdr::fmt_tokens(tok_in),
        crate::chathdr::fmt_tokens(tok_out)
    );
    if cost_microusd > 0 {
        s.push_str(" \u{00b7} ");
        s.push_str(&crate::chatsummary::fmt_cost(cost_microusd));
    }
    Some(s)
}

/// The trailer as one card line: the body's one-space indent, in the muted
/// ink the system voice uses.
pub(crate) fn trailer_line(usage: (u64, u64, u64)) -> Option<CardLine> {
    let muted = crew_theme::theme().text_muted;
    trailer_text(usage.0, usage.1, usage.2).map(|s| {
        format!(" {s}")
            .chars()
            .map(|c| plain(c, muted, false))
            .collect()
    })
}

#[cfg(test)]
#[path = "chatusage_tests.rs"]
mod tests;
