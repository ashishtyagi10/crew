//! Gantt-style timeline for the folded swarm record: one bar row per task
//! that ever started, mapping its [start, end] offset within the run onto a
//! fixed-width bar. Emitted as a fenced code block so the markdown preview
//! keeps it monospaced. Pure — `chatswarmrec` feeds it span data.

use crate::chattime::fmt_elapsed;
use crate::chatwidth::{fit_end, str_w};

/// Bar width in cells — fixed so bars stay comparable across runs. Worst-case
/// row width is TITLE_MAX + 2 + BAR_W = 36 columns; on panes narrower than
/// that the md code renderer word-wraps rows and the Gantt alignment degrades
/// (the durable record can't know the widths it will be re-rendered at).
const BAR_W: u64 = 20;
/// Widest a title may render before it is clipped.
const TITLE_MAX: usize = 14;

/// Render the timeline for `items` (title, `Some((start_ms, end_ms))` if the
/// task ever started). Returns `None` when fewer than two tasks have timing
/// or the run span is zero — a lone bar carries no concurrency information.
pub(crate) fn timeline_block(items: &[(String, Option<(u64, u64)>)]) -> Option<String> {
    let timed: Vec<(&str, u64, u64)> = items
        .iter()
        .filter_map(|(t, s)| s.map(|(a, b)| (t.as_str(), a, b)))
        .collect();
    if timed.len() < 2 {
        return None;
    }
    let total = timed.iter().map(|&(_, _, e)| e).max()?;
    if total == 0 {
        return None;
    }
    let name_w = timed
        .iter()
        .map(|(t, _, _)| str_w(t).min(TITLE_MAX))
        .max()
        .unwrap_or(0);
    let mut lines = vec![format!("timeline \u{00b7} {}", fmt_elapsed(total))];
    for (title, start, end) in &timed {
        // Titles are LLM plan output: drop control chars (an embedded newline
        // would split the row) before clipping. The 4-backtick fence below
        // keeps a title that begins with ``` from closing the block early.
        let chars: Vec<char> = title.chars().filter(|c| !c.is_control()).collect();
        let clipped: String = chars[..fit_end(&chars, 0, TITLE_MAX)].iter().collect();
        let pad = name_w.saturating_sub(str_w(&clipped));
        lines.push(format!(
            "{clipped}{:pad$}  {}",
            "",
            bar(*start, *end, total)
        ));
    }
    Some(format!("````\n{}\n````", lines.join("\n")))
}

/// One task's bar: start floors, end ceils (so brief tasks stay visible),
/// minimum one filled cell.
fn bar(start: u64, end: u64, total: u64) -> String {
    let s = ((start * BAR_W / total) as usize).min(BAR_W as usize - 1);
    let e = (end.max(start) * BAR_W)
        .div_ceil(total)
        .clamp(s as u64 + 1, BAR_W) as usize;
    (0..BAR_W as usize)
        .map(|i| {
            if i >= s && i < e {
                '\u{2588}'
            } else {
                '\u{00b7}'
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "chattimeline_tests.rs"]
mod tests;
