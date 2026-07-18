//! Draws the live swarm-run status line at the bottom of the chat message
//! area: one row saying what crew is doing right now — a spinner, the running
//! task's title, and a Claude-style `(elapsed · settled · +parallel)`
//! parenthetical, with the run's `(↑in ↓out)` token spend right-aligned at the
//! pane edge. The plan itself is not shown live; it lands in the transcript
//! when the run folds (`chatswarmrec`). State lives in `chatswarm`.
//!
//! The row's left text — spinner through the closing paren — is the "words"
//! the progress bar (`chatprog`) sizes itself to, so [`words_width`] is the
//! single source of truth both this line's layout and the bar's width read.
use crew_render::CellView;

use crate::chat::ChatPane;
use crate::chathdr::fmt_tokens;
use crate::chatswarm::{SwarmStatus, SwarmTask};
use crew_hive::TaskState;

/// Shown when the plan has arrived but nothing is running yet — the gap
/// before the first `AgentSpawned`, and the gap between one task settling and
/// the next spawning. The `…` is added by the layout, not carried here.
const WORKING: &str = "Working";
/// Below this width the elapsed piece is dropped from the parenthetical (the
/// title and counter need the room). The counter is never dropped: it's the
/// reason the line exists.
const ELAPSED_MIN_COLS: u16 = 16;
/// Columns the spinner claims at the pane's left edge. Everything else — the
/// rest of the left text, and the right-aligned tokens — is placed relative to
/// it. The spinner is asserted single-column in the tests.
const SPINNER_W: u16 = 1;

/// The elapsed piece: `4m 12s` past a minute, else `12s`. Terser than the
/// folded record's `fmt_elapsed` (which keeps a decimal) — the live line wants
/// a glanceable clock, not a precise one.
fn fmt_elapsed_short(secs: u64) -> String {
    if secs >= 60 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

/// Truncate `s` to at most `max_w` display columns using the same strict rule
/// as [`crate::chatwidth::place_row`] — a glyph that would straddle `max_w` is
/// dropped, never forced through — and return the kept prefix with its exact
/// display width. Pre-clamping here (rather than leaning on `place_row`'s
/// `max_col` at draw time) is what lets [`layout`] know the left text's true
/// width for the bar and the right-aligned tokens.
fn clamp(s: &str, max_w: u16) -> (String, u16) {
    let mut w = 0u16;
    let mut out = String::new();
    for c in s.chars() {
        let cw = crate::chatwidth::char_w(c) as u16;
        if cw == 0 {
            out.push(c); // zero-width marks ride along, no column cost
            continue;
        }
        if w + cw > max_w {
            break;
        }
        w += cw;
        out.push(c);
    }
    (out, w)
}

/// The task the line names: the oldest `Running` one, plus how many others are
/// running alongside it. `None` when nothing is running.
///
/// Oldest rather than newest so the line stays put under parallelism — naming
/// the most recent spawn would make it flicker between tasks as agents come
/// and go. A `Running` task always has `started` stamped (`chatswarm::apply`),
/// but unstamped ones sort last so a stamped task still wins if that changes.
fn focus(s: &SwarmStatus) -> Option<(&SwarmTask, usize)> {
    let mut running: Vec<&SwarmTask> = s
        .tasks
        .iter()
        .filter(|t| t.state == TaskState::Running)
        .collect();
    running.sort_by_key(|t| (t.started.is_none(), t.started));
    let first = *running.first()?;
    Some((first, running.len() - 1))
}

/// The parenthetical's inner text at a given richness: the pieces
/// `[elapsed?, count, +parallel?]` joined by ` · `. The count is always
/// present; `elapsed` and `parallel` are dropped by [`layout`] under width
/// pressure (elapsed first), never the count.
fn inner(elapsed: Option<&str>, count: &str, parallel: Option<&str>) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(3);
    parts.extend(elapsed);
    parts.push(count);
    parts.extend(parallel);
    parts.join(" \u{00b7} ")
}

/// The computed left text ("the words") plus the right-aligned token string.
/// `rest` is everything after the spinner (it carries its own leading space);
/// `left_w` is the whole left block's display width including the spinner, so
/// the block occupies columns `1..=left_w` and the bar can mirror it exactly.
struct Line {
    rest: String,
    left_w: u16,
    tokens: Option<String>,
}

/// Lay out the status line for `cols`, or `None` when the pane is too narrow
/// even for the bare `spinner + counter` floor. This is the single source of
/// truth: [`block_cells`] draws it, [`swarm_rows`] and [`words_width`] measure
/// it. Crucially it only returns `None` on the counter floor — every other
/// element (elapsed, title, parallel, tokens) degrades rather than failing —
/// so the row a pane budgets and the row it draws never disagree.
fn layout(pane: &ChatPane, cols: u16, now_ms: u64) -> Option<Line> {
    let s = pane.swarm.as_ref()?;
    if s.tasks.is_empty() {
        return None;
    }
    let (done, total) = s.settled();
    let count = format!("{done}/{total}");
    let count_w = crate::chatwidth::str_w(&count) as u16;

    // The bare `spinner + " " + count` floor: below it there's nothing sensible
    // left to draw. `rest` here is 1 (space) + count_w, and the spinner adds
    // SPINNER_W, so the block needs `SPINNER_W + 1 + count_w` columns — and one
    // is left as the right margin (place_row stops at `cols`), hence `+ 1`.
    if cols < SPINNER_W + 1 + count_w + 1 {
        return None;
    }

    let focused = focus(s);
    let title_src = match focused {
        Some((t, _)) => t.title.as_str(),
        None => WORKING,
    };
    let others = focused.map_or(0, |(_, n)| n);
    let parallel = (others > 0).then(|| format!("+{others}"));
    // Elapsed derives from `started` at render time — the per-frame redraw
    // while busy animates it for free — gated on `now_ms != 0` (tests pass 0
    // for determinism) and on the pane being wide enough to spare the columns.
    let elapsed = focused
        .filter(|_| now_ms != 0 && cols >= ELAPSED_MIN_COLS)
        .and_then(|(t, _)| t.started)
        .map(|st| fmt_elapsed_short(st.elapsed().as_secs()));

    // Content columns available after the spinner, leaving one for the right
    // margin: `[2, cols)`, i.e. `cols - SPINNER_W - 1`.
    let rest_budget = cols - SPINNER_W - 1;

    // Richest form that fits, in a strict sacrifice order: elapsed drops before
    // the title truncates (the title is worth more than a live clock), then the
    // title truncates before parallel drops (`+N` is the only signal other work
    // is running), then the title goes entirely, then parallel, and the bare
    // counter is the last resort — it never drops. `…` marks the title Claude-
    // style.
    let inner_full = inner(elapsed.as_deref(), &count, parallel.as_deref());
    let inner_no_elapsed = inner(None, &count, parallel.as_deref());
    let inner_count = count.clone();

    let rest = paren_whole(title_src, &inner_full, rest_budget) // whole title + elapsed
        .or_else(|| paren_whole(title_src, &inner_no_elapsed, rest_budget)) // drop elapsed
        .or_else(|| paren_with_title(title_src, &inner_no_elapsed, rest_budget)) // clamp title
        .or_else(|| paren_bare(&inner_no_elapsed, rest_budget)) // drop title, keep +N
        .or_else(|| paren_bare(&inner_count, rest_budget)) // drop +N
        .unwrap_or_else(|| format!(" {count}")); // floor, guaranteed to fit

    let left_w = SPINNER_W + crate::chatwidth::str_w(&rest) as u16;

    // Tokens: right-aligned `(↑in ↓out)`, shown only when the run has spent
    // something and there's a clear column of gap after the left text. They're
    // the first thing to drop on a busy pane — the bar deliberately ignores
    // them and mirrors only the left words.
    let (ti, to) = s.token_totals();
    let tokens = (ti > 0 || to > 0)
        .then(|| format!("(\u{2191}{} \u{2193}{})", fmt_tokens(ti), fmt_tokens(to)))
        .filter(|t| {
            let tw = crate::chatwidth::str_w(t) as u16;
            cols >= tw && cols - tw >= left_w + 2
        });

    Some(Line {
        rest,
        left_w,
        tokens,
    })
}

/// `" {title}… ({inner})"` only when the *whole* title fits `budget` — the
/// tier that keeps a task's name intact. `None` if it would need truncating,
/// leaving the caller to try a cheaper `inner` first (drop elapsed) before
/// resorting to [`paren_with_title`], which does truncate.
fn paren_whole(title: &str, inner: &str, budget: u16) -> Option<String> {
    let s = format!(" {title}\u{2026} ({inner})");
    (crate::chatwidth::str_w(&s) as u16 <= budget).then_some(s)
}

/// `" {title}… ({inner})"` with the title truncated to fit `budget`, or `None`
/// when even a one-column title can't share the row with `({inner})`.
fn paren_with_title(title: &str, inner: &str, budget: u16) -> Option<String> {
    let inner_w = crate::chatwidth::str_w(inner) as u16;
    // Fixed punctuation around the title: leading space + "… (" + ")" = 5 cols.
    let fixed = 1 + 3 + 1;
    let title_budget = budget.checked_sub(inner_w + fixed)?;
    if title_budget == 0 {
        return None;
    }
    let (title, _) = clamp(title, title_budget);
    if title.is_empty() {
        return None;
    }
    Some(format!(" {title}\u{2026} ({inner})"))
}

/// `" ({inner})"` with no title, for panes too narrow to show one. `None` when
/// even that doesn't fit `budget`.
fn paren_bare(inner: &str, budget: u16) -> Option<String> {
    let inner_w = crate::chatwidth::str_w(inner) as u16;
    // Leading space + "(" + ")" = 3 cols.
    (inner_w + 3 <= budget).then(|| format!(" ({inner})"))
}

/// Rows the live line occupies in the message area (0 = no live run, or a pane
/// too narrow to show even the counter floor — see [`layout`]). `now_ms` is
/// irrelevant to *whether* the row exists (only the counter floor decides
/// that, and it's clock-independent), so this passes 0 and stays deterministic
/// for the layout budget in `chatplace`.
pub(crate) fn swarm_rows(pane: &ChatPane, cols: u16) -> u16 {
    layout(pane, cols, 0).is_some() as u16
}

/// The left text's display width — the columns the "words" occupy, from the
/// spinner through the closing paren. The progress bar (`chatprog`) sizes
/// itself to this so it underlines the words rather than the whole pane.
/// `None` when there's no live line.
pub(crate) fn words_width(pane: &ChatPane, cols: u16, now_ms: u64) -> Option<u16> {
    layout(pane, cols, now_ms).map(|l| l.left_w)
}

/// Places `s` on `row` starting at `*col`, advancing by display width and
/// never emitting a cell at or beyond `max_col` — delegated to
/// `chatwidth::place_row`, which advances by `char_w` and skips zero-width
/// marks, so `max_col` is enforced structurally.
fn push_str(
    v: &mut Vec<CellView>,
    col: &mut u16,
    row: u16,
    s: &str,
    fg: (u8, u8, u8),
    max_col: u16,
) {
    let bg = crew_theme::theme().page_bg;
    *col = crate::chatwidth::place_row(*col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        v.push(CellView {
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

/// Render the status line at `top_row`. `now_ms` drives the spinner (0 in
/// tests = first frame, and suppresses elapsed so tests stay deterministic).
pub(crate) fn block_cells(pane: &ChatPane, cols: u16, top_row: u16, now_ms: u64) -> Vec<CellView> {
    let Some(line) = layout(pane, cols, now_ms) else {
        return Vec::new();
    };
    let theme = crew_theme::theme();
    let mut v = Vec::new();

    // Spinner in the accent, at the left margin.
    let f = (now_ms / 120) as usize % crate::update::SPINNER.len();
    let mut col = 1u16;
    push_str(
        &mut v,
        &mut col,
        top_row,
        &crate::update::SPINNER[f].to_string(),
        crate::palette::accent(),
        cols,
    );

    // The words (title + parenthetical), muted. Pre-clamped in `layout`, so the
    // pane-edge `max_col` here is a backstop, not the clamp.
    push_str(
        &mut v,
        &mut col,
        top_row,
        &line.rest,
        theme.text_muted,
        cols,
    );

    // Tokens, right-aligned at the pane edge, muted.
    if let Some(tokens) = &line.tokens {
        let tw = crate::chatwidth::str_w(tokens) as u16;
        let mut tcol = cols - tw;
        push_str(&mut v, &mut tcol, top_row, tokens, theme.text_muted, cols);
    }
    v
}

#[cfg(test)]
#[path = "chatswarmview_tests.rs"]
mod tests;
