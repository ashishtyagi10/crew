//! Fitting the crew pane's summary footer into the width it has: formatting
//! each number short, joining segments, and dropping the least important ones
//! when the pane is narrow.
//!
//! Split from [`crate::chatsummary`] for the line cap, along the line between
//! deciding WHAT the footer says and making it fit.
use crate::chatsummary::*;
use crew_plugin::AgentInfo;
use std::collections::HashMap;

/// A model slug trimmed to its last path segment: `anthropic/claude-sonnet-5`
/// → `claude-sonnet-5`, so provider prefixes don't crowd the line.
pub(crate) fn short_model(model: &str) -> &str {
    model.rsplit('/').next().unwrap_or(model)
}

/// `$0.129` under $10, `$12.35` above — micro-USD in, display string out.
/// Shared with the per-reply trailer (`chatusage`), so a reply's cost and the
/// footer's running total always read in the same format.
pub(crate) fn fmt_cost(microusd: u64) -> String {
    let d = microusd as f64 / 1_000_000.0;
    if d < 10.0 {
        format!("${d:.3}")
    } else {
        format!("${d:.2}")
    }
}

/// `3h52m` under a day, `3d23h` from one up — window countdowns.
pub(crate) fn fmt_left(ms: u64) -> String {
    let mins = ms / 60_000;
    let (d, h, m) = (mins / 1_440, (mins % 1_440) / 60, mins % 60);
    if d > 0 {
        format!("{d}d{h}h")
    } else {
        format!("{h}h{m:02}m")
    }
}

/// The meter glyphs. Named because [`gradient_meters`] finds the gauges by
/// glyph after the line is joined, and a bar drawn with one character and
/// re-lit by another would silently stop being a gradient.
pub(crate) const FILLED: char = '\u{2593}';

pub(crate) const EMPTY: char = '\u{2591}';

/// An 8-cell dithered meter: `▓` filled, `░` empty. 1-99% always shows at
/// least one of each so "almost empty" and "almost full" stay legible.
pub(crate) fn bar(pct: u8) -> String {
    const W: usize = 8;
    let filled = (usize::from(pct.min(100)) * W + 50) / 100;
    let filled = match pct {
        0 => 0,
        1..=99 => filled.clamp(1, W - 1),
        _ => W,
    };
    String::from(FILLED).repeat(filled) + &String::from(EMPTY).repeat(W - filled)
}

/// Drop the least important segments until the joined line fits `cols`.
///
/// `place_row` clips whatever overruns, which is silent and drops from the
/// RIGHT — so a narrow pane lost the spend meter while keeping the branch
/// name, purely because of where each sat in the line. Dropping by stated
/// priority instead means a 40-column pane loses the directory and the
/// branch, not the numbers the line exists to show.
///
/// `keep` is an index list into `segs`, most important first. Anything not
/// named is treated as least important and goes first.
pub(crate) fn budget(segs: Vec<(Seg, u8)>, cols: usize) -> Vec<Seg> {
    let width = |v: &[(Seg, u8)]| {
        v.iter().map(|((s, _), _)| s.chars().count()).sum::<usize>() + 3 * v.len().saturating_sub(1)
    };
    let mut alive = segs;
    while width(&alive) > cols && alive.len() > 1 {
        // Highest number is least important; ties break toward the right, so
        // a line never loses its leading identity to a trailing detail.
        let victim = alive
            .iter()
            .enumerate()
            .max_by_key(|(i, (_, p))| (*p, *i))
            .map(|(i, _)| i);
        match victim {
            Some(i) => {
                alive.remove(i);
            }
            None => break,
        }
    }
    alive.into_iter().map(|(seg, _)| seg).collect()
}

/// Segment importance for [`budget`], lowest kept longest. Who is answering
/// outranks what it cost, which outranks where and on what branch.
pub(crate) const P_ROSTER: u8 = 0;

pub(crate) const P_TOKENS: u8 = 1;

pub(crate) const P_COST: u8 = 2;

pub(crate) const P_BRANCH: u8 = 3;

pub(crate) const P_CWD: u8 = 4;

/// Join colored segments with a muted ` | `, then explode to per-char cells.
pub(crate) fn join(segs: &[Seg]) -> Vec<(char, Fg)> {
    join_with(segs, " | ")
}

/// [`join`] with an explicit separator. Line 3 keeps the lighter `·` it has
/// always used — it is one thought with asides, not a row of readings.
pub(crate) fn join_with(segs: &[Seg], sep: &str) -> Vec<(char, Fg)> {
    let muted = crew_theme::theme().text_muted;
    let mut out = Vec::new();
    for (i, (s, fg)) in segs.iter().enumerate() {
        if i > 0 {
            out.extend(sep.chars().map(|c| (c, muted)));
        }
        out.extend(s.chars().map(|c| (c, *fg)));
    }
    out
}

/// The tightest remaining context across agents with a known window, as a
/// fill percentage — the agent nearest its ceiling is the one that matters.
pub(crate) fn ctx_fill(agents: &[AgentInfo], ctx: &HashMap<String, u64>) -> Option<u8> {
    let mut max_fill: Option<u8> = None;
    for a in agents {
        let Some(limit) = crate::ctxlimit::context_limit(&a.model).filter(|&l| l > 0) else {
            continue;
        };
        let used = ctx.get(&a.name).copied().unwrap_or(0);
        let fill = ((used.saturating_mul(100)) / limit).min(100) as u8;
        max_fill = Some(max_fill.map_or(fill, |m| m.max(fill)));
    }
    max_fill
}
