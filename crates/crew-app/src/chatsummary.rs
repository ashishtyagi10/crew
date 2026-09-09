//! The crew pane's statusline footer: three colored lines rendered directly
//! BELOW the input composer (Claude-Code footer style). Line 1 is identity &
//! spend (model/roster · branch · $cost · token split), line 2 is the rolling
//! 5h/7d usage windows plus budget & context bars, and line 3 is the live
//! routing mode (swarm vs. `@agent` relay) and the agents working right now
//! as segment badges (`summaryroute`), then the running-task/hint tail. The
//! builder (`footer_lines_with`) is pure so it unit-tests without a live
//! pane; `summary_rows`/`summary_art` gate the height and place the rows. The line-2 meters are drawn, not spelled — see
//! `draw_meters` and [`crate::plot::meter`].
use crew_plugin::AgentInfo;
use std::collections::HashMap;

pub(crate) use crate::summaryfit::*;
pub(crate) use crate::summarymeter::*;
pub(crate) use crate::summaryroute::{lift, route_line, FCell};

use crate::chat::ChatPane;
use crate::chathdr::fmt_tokens;

/// The composer only becomes the bordered fieldset at this height (mirrors
/// `chatinput::composer_rows`). We reserve the summary row only well clear of
/// that threshold — `rows >= 8` — so pushing the composer up by one never flips
/// it back to the bare single-row prompt.
const MIN_ROWS: u16 = 8;

pub(crate) type Fg = (u8, u8, u8);
pub(crate) type Seg = (String, Fg);

/// What is running, for line 3 — `None` when nothing is.
///
/// Ids, not just a count, because `/stop #n` cancels one and the id is the
/// only way to name it. Past three, the ids stop fitting and the count is the
/// information; anyone at that point wants `/stop` (all) anyway.
pub(crate) fn running_seg(ids: &[u64], cols: usize) -> Option<(String, Option<String>)> {
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
pub(crate) fn roster_seg(agents: &[AgentInfo]) -> Option<String> {
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
    /// Names of agents thinking RIGHT NOW (`ChatPane::active_names`) —
    /// `Activity` events name agents for both relay hops and swarm tasks,
    /// so this one field covers both. Empty when idle.
    pub active: Vec<&'a str>,
    /// Where this pane's broker operates, already `~`-abbreviated.
    pub cwd: Option<&'a str>,
    pub windows: crate::usageledger::Windows,
    /// The pane's animated numbers — borrowed, since the footer is rendered
    /// from an immutable pane and the counters use interior mutability.
    pub readouts: &'a crate::readout::Readouts,
    /// Each agent's last token burst, for the working badges' pulse
    /// (`summarypulse`); `None` renders every badge on its plain roster colour.
    pub pulse: Option<crate::summarypulse::Pulses<'a>>,
}

/// The Claude-Code-style statusline: up to three colored lines (identity &
/// spend / rolling windows & bars / routing mode & hints). Pure — everything
/// it shows arrives via `FooterCtx`, so it unit-tests without a live pane.
#[cfg(test)]
pub(crate) fn footer_lines(fc: &FooterCtx, cols: usize) -> Vec<Vec<(char, Fg)>> {
    footer_text_lines(fc, cols, &mut Vec::new())
}

/// [`footer_lines_with`] as text and ink only — the badge blocks dropped —
/// for the tests that read what the footer SAYS. `summaryroute_tests` reads
/// the blocks.
#[cfg(test)]
pub(crate) fn footer_text_lines(
    fc: &FooterCtx,
    cols: usize,
    meters: &mut Vec<f32>,
) -> Vec<Vec<(char, Fg)>> {
    footer_lines_with(fc, cols, meters)
        .into_iter()
        .map(|l| l.into_iter().map(|(c, fg, _)| (c, fg)).collect())
        .collect()
}

/// [`footer_lines`], also reporting each line-2 meter's exact fill in
/// `meters`, left to right.
///
/// The reserved glyph run says where a meter goes and how long it is; it
/// cannot say where the fill lands, because eight cells only have eight
/// stops. The drawn meter needs the real number ([`crate::plot::meter`]).
pub(crate) fn footer_lines_with(
    fc: &FooterCtx,
    cols: usize,
    meters: &mut Vec<f32>,
) -> Vec<Vec<FCell>> {
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
            let pct = fc.readouts.bar_5h.tick(f64::from(pct), now);
            meters.push((pct / 100.0).clamp(0.0, 1.0) as f32);
            let pct = pct.round() as u8;
            l2.push(((format!("{} {pct}% (5h)", bar(pct)), muted), 2));
        }
        if let Some(fill) = ctx_fill(fc.agents, fc.ctx) {
            let fill = fc.readouts.ctx.tick(f64::from(fill), now);
            meters.push((fill / 100.0).clamp(0.0, 1.0) as f32);
            let fill = fill.round() as u8;
            l2.push(((format!("{} {fill}% (ctx)", bar(fill)), muted), 3));
        }
    }

    let l1 = budget(l1, cols);
    // The budget drops the meters first on a narrow pane. Their fractions go
    // with them, or the drawn meters would be paired with the wrong runs.
    let kept = l2.len();
    let l2 = budget(l2, cols);
    if l2.len() < kept {
        meters.truncate(l2.iter().filter(|(text, _)| text.contains(FILLED)).count());
    }

    // Line 3: the routing mode and who is working, as badges, then what is
    // RUNNING or the hints — `summaryroute`.
    let l3 = route_line(fc, cols);

    // Line 1 priorities: who is answering, then the spend, then the cost,
    // then the branch; the directory is the first thing to go (it is also
    // gated on width above, so on a wide pane nothing is lost at all).
    vec![lift(join(&l1)), lift(join(&l2)), l3]
}

pub(crate) fn footer_ctx(pane: &ChatPane, now_ms: u64) -> FooterCtx<'_> {
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
        active: pane.active_names(),
        cwd: pane.cwd.as_deref(),
        windows: crate::usageledger::windows(now_ms),
        // The bursts are stamped on the animation clock, not the wall clock
        // `now_ms` carries for the usage windows.
        pulse: Some(crate::summarypulse::Pulses {
            map: &pane.token_pulse,
            now: crate::anim::now_ms(),
        }),
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

#[cfg(test)]
#[path = "chatsummary_tests.rs"]
mod tests;
