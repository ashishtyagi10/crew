//! The turn's time strip, drawn at the end of a receipt head's first row.
//!
//! One bar of lower-half blocks (the per-file bar's heavy rule drew as a
//! hairline, too thin to tell three colours apart), one run per phase, widths
//! proportional to the time each held, then the total and the phase that held
//! the most, in its own colour (`42s · 58% running`): four colours with no key
//! read as decoration, and the word is the key for the one that matters. It is added AFTER the reach and the tip have taken their room and
//! only where it still fits, so it never costs a row and never displaces
//! them; a head too narrow for it keeps its row exactly as it was.
use crate::chatbody::{CardLine, Color};
use crate::chatreceiptcells::push;
use crate::chatstrip::Kind;
use crate::chatwidth::str_w;
use crate::md::syntax::Token;

/// The bar's cell: a half-height block, thick enough to read each phase's
/// colour and still lower than a line of text.
pub(crate) const BLOCK: char = '\u{2584}';

/// The bar's width range, in cells.
pub(crate) const MIN: usize = 12;
pub(crate) const MAX: usize = 20;

/// `ms` as whole cells of a `w`-cell bar, largest remainder first so the
/// widths sum to `w` exactly; a phase that rounds to nothing is dropped.
pub(crate) fn widths(ms: [u64; 4], w: usize) -> [usize; 4] {
    let total: u64 = ms.iter().sum();
    let mut out = [0usize; 4];
    if total == 0 {
        return out;
    }
    let mut rem = [0u64; 4];
    for (i, m) in ms.iter().enumerate() {
        let scaled = m * w as u64;
        (out[i], rem[i]) = ((scaled / total) as usize, scaled % total);
    }
    let mut order = [0, 1, 2, 3];
    order.sort_by_key(|&i| std::cmp::Reverse(rem[i]));
    let short = w - out.iter().sum::<usize>();
    order.iter().take(short).for_each(|&i| out[i] += 1);
    out
}

/// A phase's mark. Hue-aware token inks the diff bar and code already use,
/// so a theme that tuned them tuned these; the model's own time is the
/// quiet one (the comment ink).
fn ink(k: Kind) -> Color {
    let t = match k {
        Kind::Think => Token::Comment,
        Kind::Read => Token::Hunk,
        Kind::Edit => Token::Keyword,
        Kind::Run => Token::Str,
    };
    crate::chatreceiptcells::ink(t)
}

/// The phase that held the most time, as `58% running`.
pub(crate) fn most(ms: [u64; 4]) -> Option<(Kind, String)> {
    let total = ms.iter().sum::<u64>().max(1);
    let k = Kind::ALL.into_iter().max_by_key(|k| ms[*k as usize])?;
    let word = match k {
        Kind::Think => "thinking",
        Kind::Read => "reading",
        Kind::Edit => "editing",
        Kind::Run => "running",
    };
    Some((k, format!("{}% {word}", ms[k as usize] * 100 / total)))
}

/// The strip and the total at the end of `out`'s first row, when it fits.
pub(crate) fn strip_on(out: &mut [CardLine], ms: [u64; 4], cols: usize) {
    let total = ms.iter().sum::<u64>();
    let Some(first) = out.first_mut().filter(|_| total >= 1_000) else {
        return;
    };
    let label = crate::runclock::ladder(total / 1_000);
    let Some((k, said)) = most(ms) else { return };
    let label_w = str_w(&label) + 3 + str_w(&said);
    // The last column stays margin; two before the bar, one before the total.
    let room = cols.saturating_sub(1).saturating_sub(first.len());
    let Some(bar) = room.checked_sub(3 + label_w).filter(|b| *b >= MIN) else {
        return;
    };
    push(first, "  ", (0, 0, 0));
    let w = widths(ms, bar.min(MAX));
    for k in Kind::ALL {
        let c = crate::chatbody::plain(BLOCK, ink(k), false);
        first.extend(std::iter::repeat_n(c, w[k as usize]));
    }
    let muted = crew_theme::theme().text_muted;
    push(first, &format!(" {label} \u{b7} "), muted);
    push(first, &said, ink(k));
}

#[cfg(test)]
#[path = "chatreceiptstrip_tests.rs"]
mod tests;
