//! What a settled tool block's calls WERE, for its one summary row (see
//! [`crate::chattoolview::summary`]). `▸ 11 tool calls · 37.7s` was a count
//! with no shape: a turn that read eleven files and one that edited eleven
//! looked alike, and where its time went was nowhere. So the calls are
//! named by the phase they put the turn in (`chatphase::of_tool`, the same
//! sort the code line's word uses) — `6 read · 3 edit · 2 run` — and a strip
//! after the text gives each phase cells in proportion to its share of the
//! time, in an ink of its own. It shares the summary's row, so the turn
//! gains no row; motion plays no part, so with motion off it says the same.
use crate::chatbody::{plain, CardLine, Color};
use crate::chatphase::{of_tool, Phase};
use crate::chattool::ToolBlock;
use crate::chattoolkind::LineKind;
use crate::md::syntax::Token;

/// The strip's width in cells: wide enough that a phase with a twelfth of
/// the time still shows, short enough to sit after the text at 470 px.
pub(crate) const STRIP_CELLS: usize = 12;

/// The strip's cell: a heavy rule, which the box-drawing path draws whole.
const CELL: char = '\u{2501}';

/// The word a phase's calls are counted under.
fn noun(p: Phase) -> &'static str {
    match p {
        Phase::Tracing => "read",
        Phase::Rewriting => "edit",
        Phase::Compiling => "run",
        Phase::Decrypting => "git",
        Phase::Reaching => "web",
        Phase::Replicating => "agent",
        _ => "other",
    }
}

/// The calls grouped by [`noun`], in the order each first ran — the turn's
/// own sequence (read, then edit, then run) — with the count and the ms.
pub(crate) fn kinds(b: &ToolBlock) -> Vec<(&'static str, usize, u64)> {
    let mut out: Vec<(&'static str, usize, u64)> = Vec::new();
    for l in b.lines.iter().filter(|l| l.kind == LineKind::Tool) {
        let n = noun(of_tool(&l.label));
        let ms = l.done.as_ref().map_or(0, |d| d.ms);
        match out.iter_mut().find(|k| k.0 == n) {
            Some(k) => (k.1, k.2) = (k.1 + 1, k.2 + ms),
            None => out.push((n, 1, ms)),
        }
    }
    out
}

/// `6 read · 3 edit · 2 run`; `None` for a block with no calls.
pub(crate) fn kinds_text(b: &ToolBlock) -> Option<String> {
    let k = kinds(b);
    (!k.is_empty()).then(|| {
        k.iter()
            .map(|(n, c, _)| format!("{c} {n}"))
            .collect::<Vec<_>>()
            .join(" \u{b7} ")
    })
}

/// Each kind's cells out of `width`, by its share of the time — largest
/// remainder, so the cells always add up to `width` and no kind is a cell
/// richer than its share. Empty when nothing was timed.
pub(crate) fn widths(b: &ToolBlock, width: usize) -> Vec<(&'static str, usize)> {
    let k = kinds(b);
    let total: u64 = k.iter().map(|x| x.2).sum();
    if total == 0 || width == 0 {
        return Vec::new();
    }
    let exact: Vec<u64> = k.iter().map(|x| x.2 * width as u64).collect();
    let mut cells: Vec<usize> = exact.iter().map(|e| (e / total) as usize).collect();
    let mut order: Vec<usize> = (0..k.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(exact[i] % total));
    let short = width - cells.iter().sum::<usize>();
    for &i in order.iter().take(short) {
        cells[i] += 1;
    }
    k.iter().zip(cells).map(|(x, c)| (x.0, c)).collect()
}

/// A kind's ink. Reading changed nothing, so it wears the summary's own
/// muted ink; edits the added-line green; a run the keyword hue — the code
/// inks, which every palette already keeps legible on its card, and which
/// stay apart from each other where a read's teal beside the green did not.
fn ink(noun: &str) -> Color {
    let token = match noun {
        "read" => return crew_theme::theme().text_muted,
        "edit" => Token::Added,
        "run" => Token::Keyword,
        "git" => Token::Hunk,
        "web" => Token::Number,
        "agent" => Token::Type,
        _ => Token::Str,
    };
    crate::chatink::token_fg(token)
}

/// The drawn strip, a two-cell gap before it; empty when nothing was timed.
pub(crate) fn strip(b: &ToolBlock) -> CardLine {
    let w = widths(b, STRIP_CELLS);
    if w.is_empty() {
        return Vec::new();
    }
    let gap = crew_theme::theme().text_muted;
    let mut out: CardLine = "  ".chars().map(|c| plain(c, gap, false)).collect();
    for (n, cells) in w {
        out.extend(std::iter::repeat_n(plain(CELL, ink(n), false), cells));
    }
    out
}

#[cfg(test)]
#[path = "chattoolstrip_tests.rs"]
mod tests;
