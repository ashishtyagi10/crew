//! The crew pane's statusline footer: three colored lines rendered directly
//! BELOW the input composer (Claude-Code footer style). Line 1 is identity &
//! spend (model/roster · branch · $cost · token split), line 2 is the rolling
//! 5h/7d usage windows plus budget & context bars, and line 3 is the live
//! routing mode (swarm vs. `@agent` relay) followed by the hints that used to
//! crowd the composer placeholder. The builder (`footer_lines`) is pure so it
//! unit-tests without a live pane; `summary_rows`/`summary_cells` gate the
//! height and place the rows.
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

type Fg = (u8, u8, u8);
type Seg = (String, Fg);

pub(crate) struct FooterCtx<'a> {
    pub agents: &'a [AgentInfo],
    pub ctx: &'a HashMap<String, u64>,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
    pub branch: Option<&'a str>,
    /// The composer's current text, for the live routing-mode line.
    pub input: &'a str,
    pub windows: crate::usageledger::Windows,
}

/// `$0.129` under $10, `$12.35` above — micro-USD in, display string out.
fn fmt_cost(microusd: u64) -> String {
    let d = microusd as f64 / 1_000_000.0;
    if d < 10.0 {
        format!("${d:.3}")
    } else {
        format!("${d:.2}")
    }
}

/// `3h52m` under a day, `3d23h` from one up — window countdowns.
fn fmt_left(ms: u64) -> String {
    let mins = ms / 60_000;
    let (d, h, m) = (mins / 1_440, (mins % 1_440) / 60, mins % 60);
    if d > 0 {
        format!("{d}d{h}h")
    } else {
        format!("{h}h{m:02}m")
    }
}

/// An 8-cell dithered meter: `▓` filled, `░` empty. 1-99% always shows at
/// least one of each so "almost empty" and "almost full" stay legible.
fn bar(pct: u8) -> String {
    const W: usize = 8;
    let filled = (usize::from(pct.min(100)) * W + 50) / 100;
    let filled = match pct {
        0 => 0,
        1..=99 => filled.clamp(1, W - 1),
        _ => W,
    };
    "\u{2593}".repeat(filled) + &"\u{2591}".repeat(W - filled)
}

/// Join colored segments with a muted ` | `, then explode to per-char cells.
fn join(segs: &[Seg]) -> Vec<(char, Fg)> {
    let muted = crew_theme::theme().text_muted;
    let mut out = Vec::new();
    for (i, (s, fg)) in segs.iter().enumerate() {
        if i > 0 {
            out.extend(" | ".chars().map(|c| (c, muted)));
        }
        out.extend(s.chars().map(|c| (c, *fg)));
    }
    out
}

/// The tightest remaining context across agents with a known window, as a
/// fill percentage — the agent nearest its ceiling is the one that matters.
fn ctx_fill(agents: &[AgentInfo], ctx: &HashMap<String, u64>) -> Option<u8> {
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

/// The Claude-Code-style statusline: up to three colored lines (identity &
/// spend / rolling windows & bars / routing mode & hints). Pure — everything
/// it shows arrives via `FooterCtx`, so it unit-tests without a live pane.
pub(crate) fn footer_lines(fc: &FooterCtx, cols: usize) -> Vec<Vec<(char, Fg)>> {
    let th = crew_theme::theme();
    let (cyan, blue, green, magenta, yellow) = (
        th.ansi[14],
        th.ansi[12],
        th.ansi[10],
        th.ansi[13],
        th.ansi[11],
    );
    let muted = th.text_muted;

    // Line 1: model | branch | $cost | in/out.
    let mut l1: Vec<Seg> = Vec::new();
    let mut models: Vec<&str> = Vec::new();
    for a in fc.agents {
        let m = short_model(&a.model);
        if !models.contains(&m) {
            models.push(m);
        }
    }
    match models.as_slice() {
        [] => {}
        [one] => l1.push(((*one).to_string(), cyan)),
        many => l1.push((format!("{} agents", many.len()), cyan)),
    }
    if let Some(b) = fc.branch {
        l1.push((b.to_string(), yellow));
    }
    if fc.cost_microusd > 0 {
        l1.push((fmt_cost(fc.cost_microusd), green));
    }
    l1.push((
        format!(
            "{} in / {} out",
            fmt_tokens(fc.tok_in),
            fmt_tokens(fc.tok_out)
        ),
        magenta,
    ));

    // Line 2: 5h/7d countdowns, then budget + context bars (bars are the
    // first thing to go on a narrow pane).
    let mut l2: Vec<Seg> = Vec::new();
    let left = |w: Option<crate::usageledger::WindowStat>| {
        w.map_or("--".to_string(), |w| fmt_left(w.left_ms))
    };
    l2.push((format!("5h:{}", left(fc.windows.five_h)), blue));
    l2.push((format!("7d:{}", left(fc.windows.seven_d)), blue));
    if cols >= 60 {
        if let Some(w) = fc.windows.five_h {
            let pct = ((w.spent.saturating_mul(100)) / w.budget.max(1)).min(100) as u8;
            l2.push((format!("{} {pct}% (5h)", bar(pct)), muted));
        }
        if let Some(fill) = ctx_fill(fc.agents, fc.ctx) {
            l2.push((format!("{} {fill}% (ctx)", bar(fill)), muted));
        }
    }

    // Line 3: live routing mode + the hints that used to crowd the composer.
    let mode = match crate::chatinput::relay_target(fc.input, fc.agents) {
        Some(name) => format!("\u{25b6}\u{25b6} @{name} relay"),
        None => "\u{25b6}\u{25b6} swarm mode".to_string(),
    };
    let hints = " \u{00b7} / for constructs \u{00b7} @ to relay to an agent";
    let mut l3: Vec<(char, Fg)> = mode.chars().map(|c| (c, yellow)).collect();
    l3.extend(hints.chars().map(|c| (c, muted)));

    vec![join(&l1), join(&l2), l3]
}

/// The most rows the footer ever claims (identity/spend, windows/bars,
/// routing mode).
const MAX_BLOCK: u16 = 3;

fn footer_ctx(pane: &ChatPane, now_ms: u64) -> FooterCtx<'_> {
    FooterCtx {
        agents: &pane.agents,
        ctx: &pane.ctx,
        tok_in: pane.tok_in,
        tok_out: pane.tok_out,
        cost_microusd: pane.cost_microusd,
        branch: pane.git_branch.as_deref(),
        input: &pane.input,
        windows: crate::usageledger::windows(now_ms),
    }
}

/// Rows the footer claims at the very bottom of a `cols`×`rows` pane.
///
/// `0` when the pane is too short/narrow (below `MIN_ROWS`). Otherwise the
/// footer grows one row at a time with pane height — `rows - (MIN_ROWS - 1)`
/// rows, capped at [`MAX_BLOCK`]. The `rows - (MIN_ROWS-1)` budget guarantees
/// the composer is always measured against at least `MIN_ROWS - 1` rows, so
/// growing the footer never shrinks the composer below its bordered
/// threshold. The single source both `chatplace::grants` (row budget) and
/// `chatview::cells` (placement) read, so the reserved rows and the drawn
/// rows never disagree.
pub(crate) fn summary_rows(pane: &ChatPane, cols: u16, rows: u16) -> u16 {
    if rows < MIN_ROWS || cols < 6 {
        return 0;
    }
    let _ = pane;
    // Always 3 lines when the budget allows — line 1 alone otherwise. The
    // `rows - (MIN_ROWS-1)` budget keeps the composer's bordered threshold.
    let budget = rows - (MIN_ROWS - 1);
    budget.min(MAX_BLOCK)
}

/// Render the footer's `height` lines starting at `top`, each indented one
/// column so it reads as a footer rather than a continuation of the composer
/// border. Clipped to `cols`; empty when `height` is 0.
pub(crate) fn summary_cells(pane: &ChatPane, cols: u16, top: u16, height: u16) -> Vec<CellView> {
    if height == 0 {
        return Vec::new();
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let lines = footer_lines(&footer_ctx(pane, now_ms), cols as usize);
    let bg = crew_theme::theme().page_bg;
    let mut cells = Vec::new();
    for (i, line) in lines.into_iter().take(height as usize).enumerate() {
        let row = top + i as u16;
        crate::chatwidth::place_row(1, cols, line.into_iter(), |x, c, fg| {
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
