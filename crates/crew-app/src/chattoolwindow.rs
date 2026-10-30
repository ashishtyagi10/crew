//! While a turn runs, its tool block shows the last few calls and counts the
//! rest on one row you can open.
//!
//! A live block drew one row for every call, so a thirty-call Claude Code
//! turn grew a thirty-row wall under its card while it worked — the tool-call
//! list the user turned down on 2026-10-15, and "long things fold into a
//! count you can open" is the smith pane's own rule. Claude Code folds older
//! calls the same way ("+N more tool uses"); Codex shows the current command
//! and a count. A settled block keeps its summary line (`chattoolview`).
use crate::chatbody::{plain, CardLine};
use crate::chattool::ToolBlock;
use crate::chattoolline::glyph;
use crate::glyphs::Glyph;

/// The latest calls a live block keeps in view.
pub(crate) const SHOWN: usize = 3;

/// A live block's head row, if it has one, and how many of its first lines
/// fold under it: none while it is short, every line but the last
/// [`SHOWN`] while folded, none once clicked open (the head stays, to close
/// it again). Settled blocks have their summary line instead.
pub(crate) fn fold(b: &ToolBlock) -> Option<usize> {
    if b.settled || b.lines.len() <= SHOWN + 1 {
        return None;
    }
    Some(if b.expanded { 0 } else { b.lines.len() - SHOWN })
}

/// `  ▸ 9 earlier · 1 running · 2 failed` — the folded lines counted, the
/// failures in the removed ink; `▾ all 12 calls` once open.
pub(crate) fn head(b: &ToolBlock, hidden: usize, cols: usize, on: bool) -> CardLine {
    let th = crew_theme::theme();
    let early = &b.lines[..hidden];
    let running = early.iter().filter(|l| l.done.is_none()).count();
    let failed = early
        .iter()
        .filter(|l| l.done.as_ref().is_some_and(|d| !d.ok))
        .count();
    let (mark, said) = match hidden {
        0 => (Glyph::ToolOpen, format!("all {} calls", b.lines.len())),
        n => (Glyph::Tool, format!("{n} earlier")),
    };
    let mut text = format!("  {} {said}", glyph(mark, on));
    if running > 0 {
        text.push_str(&format!(" \u{00b7} {running} running"));
    }
    let mut out: CardLine = text
        .chars()
        .map(|c| plain(c, th.text_muted, false))
        .collect();
    if failed > 0 {
        let fg = crate::chatink::token_fg(crate::md::syntax::Token::Removed);
        let tail = format!(" \u{00b7} {failed} failed");
        out.extend(tail.chars().map(|c| plain(c, fg, false)));
    }
    out.truncate(cols);
    out
}

#[cfg(test)]
#[path = "chattoolwindow_tests.rs"]
mod tests;
