//! The crew pane's empty state: instead of a bare one-line hint, a fresh pane
//! introduces the crew — connection state, the detected agents with their
//! roles, and how to start (plain task, or `@agent` to pick who begins) — so
//! the first run explains itself.
use crew_plugin::AgentInfo;
use crew_render::CellView;

#[cfg(test)]
use crate::balancewrap::greedy;
use crate::balancewrap::{wrap_to, wrap_width};

/// A text row at `(row, col..)`, clipped to `cols`.
fn line(
    out: &mut Vec<CellView>,
    row: u16,
    col: u16,
    cols: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) {
    let bg = crew_theme::theme().page_bg;
    // Width-aware (see `chatwidth`): roster names can carry wide glyphs.
    crate::chatwidth::place_row(col, cols, s.chars().map(|c| (c, fg)), |x, c, fg| {
        out.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
            ..Default::default()
        });
    });
}

/// One onboarding row: its text, ink and weight.
type Row = (String, (u8, u8, u8), bool);

/// Render the onboarding block into rows `top..max_row`.
///
/// The block is composed first and fitted second, because the rows it gets
/// are whatever the composer and the footer have left: on a short tile the
/// no-provider advice used to end `automatically,` — a sentence cut by the
/// row budget with nothing to say so.
pub(crate) fn empty_cells(
    cols: u16,
    max_row: u16,
    top: u16,
    connected: bool,
    agents: &[AgentInfo],
) -> Vec<CellView> {
    let avail = max_row.saturating_sub(top + 1) as usize;
    let mut cells = Vec::new();
    for (i, (s, fg, bold)) in fit(block(cols, connected, agents, avail), avail, cols)
        .into_iter()
        .enumerate()
    {
        line(&mut cells, top + 1 + i as u16, 1, cols, &s, fg, bold);
    }
    cells
}

/// The rows the state asks for, before any budget is applied.
fn block(cols: u16, connected: bool, agents: &[AgentInfo], avail: usize) -> Vec<Row> {
    let t = crew_theme::theme();
    let muted = |s: String| (s, t.text_muted, false);
    if !connected {
        return vec![muted(
            "\u{25cb} connecting to the crew broker\u{2026}".to_string(),
        )];
    }
    if agents.is_empty() {
        let mut rows = vec![
            ("No agents available.".to_string(), t.ink, true),
            muted(String::new()),
        ];
        // The same words the broker uses (`crew_plugin::no_provider_forms`),
        // wrapped to the pane. Four wordings of this advice existed across the
        // two processes and two of them went stale for two releases; there is
        // one copy now, and this is a view of it.
        //
        // …at the LENGTH that fits. This is the first thing crew says to
        // someone with no provider, and on a quarter tile the long form was
        // cut at `paste it at a Nemotron row…` — a shorter true sentence beats
        // a longer cut one. The headline and its blank row cost two of the
        // rows the advice is measured against.
        rows.extend(advice(cols, avail.saturating_sub(rows.len())));
        return rows;
    }
    // Minimal, Claude-Code-style: a single muted hint. No roster dump and no
    // keybind table — the pane shouldn't spend rows on chrome before the first
    // task. `@agent` picks who starts; plain text runs the swarm. Wrapped: on
    // a half tile the one sentence ended `/ for comm`.
    let first = &agents[0].name;
    let hint = format!(
        "Type a task and press Enter \u{2014} @agent to pick who starts (e.g. @{first}), / for commands."
    );
    let mut rows: Vec<Row> = wrap_to(&hint, cols).into_iter().map(muted).collect();
    // Two asks that show what agent smith decides now (a spacer between,
    // the first row to go on a short tile): a swarm that checks its own
    // result, and a plan that runs only once approved.
    rows.push(muted(String::new()));
    rows.extend(wrap_to(EXAMPLES, cols).into_iter().map(muted));
    rows
}

/// The example asks under the hint — each exercises a decision the brain
/// makes, not a command.
const EXAMPLES: &str = "Try \u{201c}make the tests pass\u{201d} \u{2014} a swarm that \
    verifies its own result \u{2014} or \u{201c}draft a plan first\u{201d} \u{2014} \
    nothing runs until you approve.";

/// The longest form of the no-provider advice that fits `avail` rows at this
/// width, wrapped — the shortest one if none of them do, since something
/// whole and too long beats something cut.
fn advice(cols: u16, avail: usize) -> Vec<Row> {
    let t = crew_theme::theme();
    let forms = crew_plugin::no_provider_forms();
    let mut wrapped = forms.iter().map(|f| wrap_to(f, cols));
    // The shortest is the floor: something whole and too long beats something
    // cut, so a pane with room for none of them still gets a sentence.
    let shortest = || forms.last().map(|f| wrap_to(f, cols)).unwrap_or_default();
    let pick = wrapped
        .find(|rows| rows.len() <= avail)
        .unwrap_or_else(shortest);
    pick.into_iter().map(|s| (s, t.text_muted, false)).collect()
}

/// Fit `block` into `avail` rows: the blank spacers go first, and if the
/// words still do not fit the last row that does is cut and marked.
fn fit(mut block: Vec<Row>, avail: usize, cols: u16) -> Vec<Row> {
    if block.len() > avail {
        block.retain(|r| !r.0.is_empty());
    }
    if block.len() > avail {
        block.truncate(avail);
        if let Some(last) = block.last_mut() {
            let w = wrap_width(cols);
            let mut s = crate::chatwidth::clip_w(&last.0, w.saturating_sub(1));
            if !s.ends_with('\u{2026}') {
                s.push('\u{2026}');
            }
            last.0 = s;
        }
    }
    block
}

#[cfg(test)]
#[path = "chatempty_tests.rs"]
mod tests;
