use super::*;

fn agents(names: &[(&str, &str)]) -> Vec<AgentInfo> {
    names
        .iter()
        .map(|(n, r)| AgentInfo {
            name: (*n).into(),
            role: (*r).into(),
            model: String::new(),
        })
        .collect()
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn connecting_state_says_so() {
    let cells = empty_cells(80, 20, 2, false, &[]);
    assert!(row_text(&cells, 3).contains("connecting"));
}

/// The pane shows the SAME advice the broker gives, wrapped. Asserted by
/// reassembling the rows and comparing against the shared source: four
/// wordings of this used to exist across two processes, and the two the tests
/// did not pin went stale for two releases.
#[test]
fn missing_agents_explain_the_fix() {
    let cells = empty_cells(80, 20, 2, true, &[]);
    assert!(row_text(&cells, 3).contains("No agents"));
    let shown: String = (5..9)
        .map(|r| row_text(&cells, r))
        .collect::<Vec<_>>()
        .join(" ");
    let flat: String = shown.split_whitespace().collect::<Vec<_>>().join(" ");
    let want: String = crew_plugin::no_provider_advice()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        flat.to_lowercase().contains(&want.to_lowercase()),
        "pane advice drifted from the broker's\n pane: {flat}\n want: {want}"
    );
}

/// Wrapping keeps every word and never exceeds the pane.
#[test]
fn advice_wraps_without_losing_words() {
    for cols in [20u16, 40, 80] {
        let lines = wrap_advice(crew_plugin::no_provider_advice(), cols);
        let width = (cols.saturating_sub(4)).max(12) as usize;
        for l in &lines {
            assert!(l.chars().count() <= width, "{cols}: too wide: {l}");
        }
        let rejoined: String = lines.join(" ").to_lowercase();
        for word in crew_plugin::no_provider_advice().split_whitespace() {
            assert!(
                rejoined.contains(&word.to_lowercase()),
                "{cols}: lost {word}"
            );
        }
    }
}

#[test]
fn ready_state_is_a_single_minimal_hint() {
    // Claude-Code-style: no "ready" heading, no roster dump, no keybind table —
    // just one muted hint row. The first agent's name seeds the @-example.
    let a = agents(&[("planner", "planning"), ("coder", "implementation")]);
    let cells = empty_cells(80, 20, 2, true, &a);
    let hint = row_text(&cells, 3);
    assert!(hint.contains("Type a task"), "hint missing: {hint}");
    assert!(hint.contains("@planner"), "@-example missing: {hint}");
    // The onboarding no longer spends rows on a roster or a quick-start table.
    assert!(
        cells.iter().all(|c| c.row <= 3),
        "ready onboarding must be a single row",
    );
}

#[test]
fn everything_clips_to_bounds() {
    let a = agents(&[("planner", "a-very-long-role-description")]);
    let cells = empty_cells(12, 6, 2, true, &a);
    assert!(cells.iter().all(|c| c.col < 12 && c.row < 6));
}
