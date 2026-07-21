//! The crew pane's summary footer: a muted stats block rendered directly BELOW
//! the input composer (Claude-Code footer style), consolidating what the old
//! per-agent statusline rows spread across the top. On a tall pane it is a
//! labelled multi-row block (`summary_block`) — model, context (used/limit ·
//! % left), usage (tokens · turns), and agents (count · avg latency); a short
//! pane collapses to the dense single line (`summary_text`). Both builders are
//! pure so they unit-test without a live pane; `summary_rows`/`summary_cells`
//! gate the height and place the rows.
use crew_plugin::AgentInfo;
use crew_render::CellView;
use std::collections::HashMap;

use crate::chat::ChatPane;
use crate::chathdr::fmt_tokens;

/// The composer only becomes the bordered fieldset at this height (mirrors
/// `chatinput::composer_rows`). We reserve the summary row only well clear of
/// that threshold — `rows >= 8` — so pushing the composer up by one never flips
/// it back to the bare single-row prompt.
const MIN_ROWS: u16 = 8;

/// A model slug trimmed to its last path segment: `anthropic/claude-sonnet-5`
/// → `claude-sonnet-5`, so provider prefixes don't crowd the line.
fn short_model(model: &str) -> &str {
    model.rsplit('/').next().unwrap_or(model)
}

/// The summary line for a pane with these `agents`, per-agent context fill
/// (`ctx` tokens by agent name), and session `tokens` spend — or `None` when
/// there's nothing yet to summarise (no agents and no spend).
///
/// Segments, in order, each shown only when it has a value:
/// - **model**: the one model when the roster shares it, else `N agents`.
/// - **context**: the *tightest* remaining window across agents whose model
///   has a known limit (`{left}% context`) — the agent nearest its ceiling is
///   the one that matters, so the minimum-remaining wins.
/// - **tokens**: `~{n} tok`, the session spend, once nonzero.
pub(crate) fn summary_text(
    agents: &[AgentInfo],
    ctx: &HashMap<String, u64>,
    tokens: u64,
) -> Option<String> {
    let mut segs: Vec<String> = Vec::new();

    // Distinct models across the roster, order-preserving.
    let mut models: Vec<&str> = Vec::new();
    for a in agents {
        let m = short_model(&a.model);
        if !models.contains(&m) {
            models.push(m);
        }
    }
    match models.as_slice() {
        [] => {}
        [one] => segs.push((*one).to_string()),
        many => segs.push(format!("{} agents", many.len())),
    }

    // Tightest remaining context across agents with a known window.
    let mut min_left: Option<u8> = None;
    for a in agents {
        let Some(limit) = crate::ctxlimit::context_limit(&a.model).filter(|&l| l > 0) else {
            continue;
        };
        let used = ctx.get(&a.name).copied().unwrap_or(0);
        let fill = ((used.saturating_mul(100)) / limit).min(100) as u8;
        let left = 100 - fill;
        min_left = Some(min_left.map_or(left, |m| m.min(left)));
    }
    if let Some(left) = min_left {
        segs.push(format!("{left}% context"));
    }

    if tokens > 0 {
        segs.push(format!("~{} tok", fmt_tokens(tokens)));
    }

    (!segs.is_empty()).then(|| segs.join(" \u{00b7} "))
}

/// The most rows the summary block ever claims (model / ctx / usage / agents).
const MAX_BLOCK: u16 = 4;

/// The multi-row stats block shown below the composer when the pane is tall
/// enough, most-important row first. Each `(label, value)` row appears only
/// when it has data, so the block is 0–4 rows:
/// - **model**: the shared model, or `mixed (N)` across a mixed roster.
/// - **ctx**: the tightest agent's window — `{used}/{limit} \u{b7} {left}% left`.
/// - **usage**: `~{n} tok` session spend, and `\u{b7} {t} turns` once turns land.
/// - **agents**: roster size, and `\u{b7} avg {s}s/reply` from reply stats.
///
/// On a shorter pane only the first few rows are shown (see `summary_rows`); a
/// one-row budget falls back to the dense [`summary_text`] line instead.
fn summary_block(
    agents: &[AgentInfo],
    ctx: &HashMap<String, u64>,
    tokens: u64,
    turns: u64,
    agent_stats: &HashMap<String, (u32, u64)>,
) -> Vec<(&'static str, String)> {
    let mut rows: Vec<(&'static str, String)> = Vec::new();

    // model: one shared name, or a count when the roster mixes models.
    let mut models: Vec<&str> = Vec::new();
    for a in agents {
        let m = short_model(&a.model);
        if !models.contains(&m) {
            models.push(m);
        }
    }
    match models.as_slice() {
        [] => {}
        [one] => rows.push(("model", (*one).to_string())),
        many => rows.push(("model", format!("mixed ({})", many.len()))),
    }

    // ctx: the agent nearest its ceiling — absolute used/limit plus % left.
    let mut tightest: Option<(u64, u64, u8)> = None; // (used, limit, left%)
    for a in agents {
        let Some(limit) = crate::ctxlimit::context_limit(&a.model).filter(|&l| l > 0) else {
            continue;
        };
        let used = ctx.get(&a.name).copied().unwrap_or(0);
        let left = (100 - ((used.saturating_mul(100)) / limit).min(100)) as u8;
        if tightest.map_or(true, |(_, _, l)| left < l) {
            tightest = Some((used, limit, left));
        }
    }
    if let Some((used, limit, left)) = tightest {
        rows.push((
            "ctx",
            format!(
                "{}/{} \u{00b7} {left}% left",
                fmt_tokens(used),
                fmt_tokens(limit)
            ),
        ));
    }

    // usage: session token spend and completed turns.
    if tokens > 0 || turns > 0 {
        let mut v = format!("~{} tok", fmt_tokens(tokens));
        if turns > 0 {
            let unit = if turns == 1 { "turn" } else { "turns" };
            v.push_str(&format!(" \u{00b7} {turns} {unit}"));
        }
        rows.push(("usage", v));
    }

    // agents: roster size and the average reply latency across it.
    if !agents.is_empty() {
        let (replies, ms): (u32, u64) = agent_stats
            .values()
            .fold((0, 0), |(r, m), (ar, am)| (r + ar, m + am));
        let mut v = agents.len().to_string();
        if replies > 0 {
            let avg_s = (ms / replies as u64) as f64 / 1000.0;
            v.push_str(&format!(" \u{00b7} avg {avg_s:.1}s/reply"));
        }
        rows.push(("agents", v));
    }

    rows
}

/// Pad each block row's label to a common width so the values align into a
/// column: `"model  claude\u{2026}"`, `"ctx    45k\u{2026}"`.
fn block_lines(rows: &[(&'static str, String)]) -> Vec<String> {
    let w = rows.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    rows.iter()
        .map(|(label, value)| format!("{label:<w$} {value}"))
        .collect()
}

/// Rows the summary block claims at the very bottom of a `cols`×`rows` pane.
///
/// `0` when the pane is too short/narrow (below `MIN_ROWS`) or there is nothing
/// to summarise. Otherwise the block grows one row at a time with pane height —
/// `rows - (MIN_ROWS - 1)` rows, capped at [`MAX_BLOCK`] and at however many
/// stat rows actually have data. The `rows - (MIN_ROWS-1)` budget guarantees
/// the composer is always measured against at least `MIN_ROWS - 1` rows, so
/// growing the block never shrinks the composer below its bordered threshold.
/// The single source both `chatplace::grants` (row budget) and `chatview::cells`
/// (placement) read, so the reserved rows and the drawn rows never disagree.
pub(crate) fn summary_rows(pane: &ChatPane, cols: u16, rows: u16) -> u16 {
    if rows < MIN_ROWS || cols < 6 {
        return 0;
    }
    let lines = summary_block(
        &pane.agents,
        &pane.ctx,
        pane.tokens,
        pane.turns,
        &pane.agent_stats,
    );
    if lines.is_empty() {
        return 0;
    }
    let budget = rows - (MIN_ROWS - 1); // ≥ 1, since rows ≥ MIN_ROWS
    (lines.len() as u16).min(budget).min(MAX_BLOCK)
}

/// Render the summary block as muted rows, `height` of them starting at `top`,
/// each indented one column so it reads as a footer rather than a continuation
/// of the composer border. A one-row budget falls back to the dense
/// [`summary_text`] line; taller budgets draw the labelled block, most-important
/// row first. Clipped to `cols`; empty when there's nothing to summarise.
pub(crate) fn summary_cells(pane: &ChatPane, cols: u16, top: u16, height: u16) -> Vec<CellView> {
    if height == 0 {
        return Vec::new();
    }
    let texts: Vec<String> = if height == 1 {
        summary_text(&pane.agents, &pane.ctx, pane.tokens)
            .into_iter()
            .collect()
    } else {
        let rows = summary_block(
            &pane.agents,
            &pane.ctx,
            pane.tokens,
            pane.turns,
            &pane.agent_stats,
        );
        block_lines(&rows)
    };
    let muted = crew_theme::theme().text_muted;
    let bg = crew_theme::theme().page_bg;
    let mut cells = Vec::new();
    for (i, text) in texts.iter().take(height as usize).enumerate() {
        let row = top + i as u16;
        crate::chatwidth::place_row(1, cols, text.chars().map(|c| (c, muted)), |x, c, fg| {
            cells.push(CellView {
                col: x,
                row,
                c,
                fg,
                bg,
                bold: false,
                italic: false,
            });
        });
    }
    cells
}

#[cfg(test)]
#[path = "chatsummary_tests.rs"]
mod tests;
