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

/// What is running, for line 3 — `None` when nothing is.
///
/// Ids, not just a count, because `/stop #n` cancels one and the id is the
/// only way to name it. Past three, the ids stop fitting and the count is the
/// information; anyone at that point wants `/stop` (all) anyway.
fn running_seg(ids: &[u64], cols: usize) -> Option<(String, Option<String>)> {
    match ids {
        [] => None,
        many if many.len() > 3 => Some((format!("{} running", many.len()), None)),
        many => {
            let list: Vec<String> = many.iter().map(|i| format!("#{i}")).collect();
            let how = (cols >= 60).then(|| format!("/stop {} to cancel", list[0]));
            Some((format!("running {}", list.join(" ")), how))
        }
    }
}

/// Who is on the roster, as line 1's leading segment. This is the whole of
/// what `/agents` used to report, minus having to ask for it — which is why
/// that construct no longer exists.
///
/// The model leads whenever one model is the answer: `shared_model` ignores
/// CLI agents (`claude`, `codex`, `opencode` report no model — the CLI
/// chooses), so specialists agreeing on `qwen3-coder-plus` show that model
/// even with CLIs on the roster, plus the count that says how many hands are
/// on deck. Only when models genuinely disagree — or nobody reports one — are
/// names the honest answer, and past three even names stop fitting and stop
/// informing; the count does both.
fn roster_seg(agents: &[AgentInfo]) -> Option<String> {
    if agents.is_empty() {
        return None;
    }
    if let Some(m) = crate::chatpalette::shared_model(agents) {
        let m = short_model(&m);
        return Some(if agents.len() == 1 {
            m.to_string()
        } else {
            format!("{m} \u{00b7} {} agents", agents.len())
        });
    }
    let names: Vec<&str> = agents.iter().map(|a| a.name.as_str()).collect();
    if names.len() > 3 {
        return Some(format!("{} agents", names.len()));
    }
    Some(names.join("\u{00b7}"))
}

pub(crate) struct FooterCtx<'a> {
    pub agents: &'a [AgentInfo],
    pub ctx: &'a HashMap<String, u64>,
    pub tok_in: u64,
    pub tok_out: u64,
    pub cost_microusd: u64,
    pub branch: Option<&'a str>,
    /// The composer's current text, for the live routing-mode line.
    pub input: &'a str,
    /// Broker task ids in flight right now (see `ChatPane::running_tasks`).
    pub running_tasks: &'a [u64],
    /// A drafted plan is waiting for enter/esc.
    pub plan_pending: bool,
    /// Where this pane's broker operates, already `~`-abbreviated.
    pub cwd: Option<&'a str>,
    pub windows: crate::usageledger::Windows,
    /// The pane's animated numbers — borrowed, since the footer is rendered
    /// from an immutable pane and the counters use interior mutability.
    pub readouts: &'a crate::readout::Readouts,
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
fn budget(segs: Vec<(Seg, u8)>, cols: usize) -> Vec<Seg> {
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
const P_ROSTER: u8 = 0;
const P_TOKENS: u8 = 1;
const P_COST: u8 = 2;
const P_BRANCH: u8 = 3;
const P_CWD: u8 = 4;

/// Join colored segments with a muted ` | `, then explode to per-char cells.
fn join(segs: &[Seg]) -> Vec<(char, Fg)> {
    join_with(segs, " | ")
}

/// [`join`] with an explicit separator. Line 3 keeps the lighter `·` it has
/// always used — it is one thought with asides, not a row of readings.
fn join_with(segs: &[Seg], sep: &str) -> Vec<(char, Fg)> {
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

    // Line 1: cwd | roster | branch | $cost | in/out. The directory leads,
    // Claude-Code style — it is the answer to "where will this actually run",
    // which is what `/cwd` existed to ask. First to go on a narrow pane: it is
    // the most stable value on the line, so it is the least costly to lose.
    let mut l1: Vec<(Seg, u8)> = Vec::new();
    if let Some(d) = fc.cwd.filter(|d| !d.is_empty()) {
        l1.push(((d.to_string(), muted), P_CWD));
    }
    if let Some(r) = roster_seg(fc.agents) {
        l1.push(((r, cyan), P_ROSTER));
    }
    if let Some(b) = fc.branch {
        l1.push(((b.to_string(), yellow), P_BRANCH));
    }
    // The numbers sweep to their new values rather than snapping — see
    // `readout`. They are read through the same clock everything else animates
    // on, and each settles the moment it arrives.
    let now = crate::anim::now_ms();
    if fc.cost_microusd > 0 {
        let shown = fc.readouts.cost.tick(fc.cost_microusd as f64, now);
        l1.push(((fmt_cost(shown.round() as u64), green), P_COST));
    }
    l1.push((
        (
            format!(
                "{} in / {} out",
                fmt_tokens(fc.readouts.tok_in.tick(fc.tok_in as f64, now).round() as u64),
                fmt_tokens(fc.readouts.tok_out.tick(fc.tok_out as f64, now).round() as u64)
            ),
            magenta,
        ),
        P_TOKENS,
    ));

    // Line 2: 5h/7d countdowns, then budget + context bars (bars are the
    // first thing to go on a narrow pane).
    let mut l2: Vec<(Seg, u8)> = Vec::new();
    let left = |w: Option<crate::usageledger::WindowStat>| {
        w.map_or("--".to_string(), |w| fmt_left(w.left_ms))
    };
    l2.push(((format!("5h:{}", left(fc.windows.five_h)), blue), 0));
    l2.push(((format!("7d:{}", left(fc.windows.seven_d)), blue), 1));
    if cols >= 60 {
        // The meters fill rather than snap: a bar that jumps two cells reads
        // as a glitch, the same bar sliding reads as a gauge.
        if let Some(w) = fc.windows.five_h {
            let pct = ((w.spent.saturating_mul(100)) / w.budget.max(1)).min(100) as u8;
            let pct = fc.readouts.bar_5h.tick(f64::from(pct), now).round() as u8;
            l2.push(((format!("{} {pct}% (5h)", bar(pct)), muted), 2));
        }
        if let Some(fill) = ctx_fill(fc.agents, fc.ctx) {
            let fill = fc.readouts.ctx.tick(f64::from(fill), now).round() as u8;
            l2.push(((format!("{} {fill}% (ctx)", bar(fill)), muted), 3));
        }
    }

    let l1 = budget(l1, cols);
    let l2 = budget(l2, cols);

    // Line 3: live routing mode, then either what is RUNNING or — when
    // nothing is — the hints. Work in flight outranks teaching: the hints are
    // for someone deciding what to type, and this is the line that used to
    // require typing `/tasks` to see.
    let mode = match crate::chatinput::relay_target(fc.input, fc.agents) {
        Some(name) => format!("\u{25b6}\u{25b6} @{name} relay"),
        None => "\u{25b6}\u{25b6} swarm mode".to_string(),
    };
    // Line 3 budgets like the others. It used to be built as one string and
    // clipped, which on a 40-column pane cut "enter runs it · esc discards
    // it" in half — teaching the user how to accept a plan and not how to
    // decline it. Half an instruction is worse than none.
    let mut l3s: Vec<(Seg, u8)> = vec![((mode, yellow), 1)];
    if fc.plan_pending {
        // A pending plan outranks everything else here: it is the only thing
        // on this line addressed TO the user. The keys go compact rather than
        // missing when the pane is narrow.
        let keys = if cols >= 60 {
            "enter runs it \u{00b7} esc discards it"
        } else {
            "enter/esc"
        };
        l3s.push((("plan ready".to_string(), green), 0));
        l3s.push(((keys.to_string(), green), 0));
    } else if let Some((what, how)) = running_seg(fc.running_tasks, cols) {
        l3s.push(((what, green), 0));
        if let Some(how) = how {
            l3s.push(((how, green), 2));
        }
    } else {
        l3s.push((("/ for constructs".to_string(), muted), 2));
        l3s.push((("@ to relay to an agent".to_string(), muted), 3));
    }
    let l3 = join_with(&budget(l3s, cols), " \u{00b7} ");

    // Line 1 priorities: who is answering, then the spend, then the cost,
    // then the branch; the directory is the first thing to go (it is also
    // gated on width above, so on a wide pane nothing is lost at all).
    vec![join(&l1), join(&l2), l3]
}

/// The most rows the footer ever claims (identity/spend, windows/bars,
/// routing mode).
const MAX_BLOCK: u16 = 3;

fn footer_ctx(pane: &ChatPane, now_ms: u64) -> FooterCtx<'_> {
    FooterCtx {
        readouts: &pane.readouts,
        agents: &pane.agents,
        ctx: &pane.ctx,
        tok_in: pane.tok_in,
        tok_out: pane.tok_out,
        cost_microusd: pane.cost_microusd,
        branch: pane.git_branch.as_deref(),
        input: &pane.input,
        running_tasks: &pane.running_tasks,
        plan_pending: pane.plan_pending,
        cwd: pane.cwd.as_deref(),
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
        crate::chatwidth::place_row(1, cols, line, |x, c, fg| {
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
