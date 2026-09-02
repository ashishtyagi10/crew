//! The crew pane's empty state: instead of a bare one-line hint, a fresh pane
//! introduces the crew — connection state, the detected agents with their
//! roles, and how to start (plain task, or `@agent` to pick who begins) — so
//! the first run explains itself.
use crew_plugin::AgentInfo;
use crew_render::CellView;

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
    for (i, (s, fg, bold)) in fit(block(cols, connected, agents), avail, cols)
        .into_iter()
        .enumerate()
    {
        line(&mut cells, top + 1 + i as u16, 1, cols, &s, fg, bold);
    }
    cells
}

/// The rows the state asks for, before any budget is applied.
fn block(cols: u16, connected: bool, agents: &[AgentInfo]) -> Vec<Row> {
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
        // The same words the broker uses (`crew_plugin::no_provider_advice`),
        // wrapped to the pane. Four wordings of this advice existed across the
        // two processes and two of them went stale for two releases; there is
        // one copy now, and this is a view of it.
        rows.extend(
            wrap_to(crew_plugin::no_provider_advice(), cols)
                .into_iter()
                .map(muted),
        );
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
    wrap_to(&hint, cols).into_iter().map(muted).collect()
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

/// Columns a wrapped onboarding line may take.
fn wrap_width(cols: u16) -> usize {
    (cols.saturating_sub(4)).max(12) as usize
}

#[cfg(test)]
#[path = "chatempty_tests.rs"]
mod tests;

/// Wrap `advice` to the pane's width, on spaces, with a sentence capital.
/// Pure so the wrapping is testable without a pane.
fn wrap_to(advice: &str, cols: u16) -> Vec<String> {
    let width = wrap_width(cols);
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in advice.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    if let Some(first) = out.first_mut() {
        let mut c = first.chars();
        if let Some(f) = c.next() {
            *first = f.to_uppercase().collect::<String>() + c.as_str();
        }
    }
    out
}
