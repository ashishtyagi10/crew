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

#[test]
fn missing_agents_explain_the_fix() {
    let cells = empty_cells(80, 20, 2, true, &[]);
    assert!(row_text(&cells, 3).contains("No agents"));
    assert!(row_text(&cells, 5).contains("OPENROUTER_API_KEY"));
}

#[test]
fn ready_state_lists_agents_with_roles_and_example() {
    let a = agents(&[("planner", "planning"), ("coder", "implementation")]);
    let cells = empty_cells(80, 20, 2, true, &a);
    assert!(row_text(&cells, 3).contains("Your crew is ready"));
    assert!(row_text(&cells, 5).contains("planner \u{2014} planning"));
    assert!(row_text(&cells, 6).contains("coder \u{2014} implementation"));
    assert!(row_text(&cells, 9).contains("Try: @planner"));
}

#[test]
fn everything_clips_to_bounds() {
    let a = agents(&[("planner", "a-very-long-role-description")]);
    let cells = empty_cells(12, 6, 2, true, &a);
    assert!(cells.iter().all(|c| c.col < 12 && c.row < 6));
}

// Quick-start hint block (iter 9). With one agent + top=2, the existing
// ready-state content ends at row 9 (see `empty_cells`'s put-call sequence:
// "ready" (3) + blank (4) + 1 agent row (5) + blank (6) + type-hint (7) +
// try-line (8), leaving row 9 as the next free row). Row 9 is left blank as
// the separator, the `─ quick start ─` rule lands on row 10, and the five
// hint rows follow on 11..=15.
const QS_RULE_ROW: u16 = 10;
const QS_FIRST_HINT_ROW: u16 = 11;
// `desc_col = 1 (indent) + 6 (longest key, "@agent"/"Ctrl+O") + 3 (gap)`.
const QS_DESC_COL: u16 = 10;
// Minimum width for quick start to render at all (without secondary clauses).
// After splitting Esc into "interrupt a running turn" + "idle: close pane",
// the longest primary is now "interrupt a running turn" (24 cols) or "@agent"'s
// full line with clause (43 cols). desc_col + 24 = 34.
const QS_BARE_WIDTH: u16 = 34;
// Minimum width for secondary clauses to render. The widest with-clause line
// is "@agent ... · ..." (43 cols). desc_col + 43 = 53.
const QS_FULL_WIDTH: u16 = 53;

#[test]
fn quick_start_shows_on_tall_wide_pane_with_accent_keys_aligned() {
    let _g = crate::palette::test_guard();
    let a = agents(&[("planner", "planning")]);
    let cells = empty_cells(80, 30, 2, true, &a);
    assert!(row_text(&cells, QS_RULE_ROW).contains("quick start"));
    let keys = ["Enter", "@agent", "Esc", "Ctrl+O", "/"];
    for (i, key) in keys.iter().enumerate() {
        let r = QS_FIRST_HINT_ROW + i as u16;
        // Key starts at col 1 in the accent color.
        let first = cells
            .iter()
            .find(|c| c.row == r && c.col == 1)
            .unwrap_or_else(|| panic!("no cell at row {r} col 1"));
        assert_eq!(first.c, key.chars().next().unwrap());
        assert_eq!(first.fg, crate::palette::accent());
        // Description is aligned at the same column for every row, muted.
        let desc = cells
            .iter()
            .find(|c| c.row == r && c.col == QS_DESC_COL)
            .unwrap_or_else(|| panic!("no description cell at row {r} col {QS_DESC_COL}"));
        assert_eq!(desc.fg, crew_theme::theme().text_muted);
    }
    // Secondary clauses render at this width: the "Enter" row's " · " clause.
    assert!(row_text(&cells, QS_FIRST_HINT_ROW).contains("type while busy to queue"));
    // No overlap with the existing empty-state content (which ends by row 9).
    assert!(cells.iter().all(|c| c.row <= 8 || c.row >= QS_RULE_ROW));
}

#[test]
fn quick_start_shows_primaries_without_clauses_in_mid_tier() {
    let _g = crate::palette::test_guard();
    let a = agents(&[("planner", "planning")]);
    // Width 45 is in the mid-tier band: 34 <= 45 < 53 (bare_w <= cols < full_w).
    // At this width, primaries render but secondary clauses (` · ` sections) do not.
    let mid_tier_cols = 45;
    let cells = empty_cells(mid_tier_cols, 30, 2, true, &a);
    assert!(row_text(&cells, QS_RULE_ROW).contains("quick start"));

    // Keys are aligned at col 1 in accent color.
    let keys = ["Enter", "@agent", "Esc", "Ctrl+O", "/"];
    for (i, key) in keys.iter().enumerate() {
        let r = QS_FIRST_HINT_ROW + i as u16;
        let first = cells
            .iter()
            .find(|c| c.row == r && c.col == 1)
            .unwrap_or_else(|| panic!("no cell at row {r} col 1"));
        assert_eq!(first.c, key.chars().next().unwrap());
        assert_eq!(first.fg, crate::palette::accent());
    }

    // Primary clauses present; secondary clauses absent (no " · " joins).
    assert!(row_text(&cells, QS_FIRST_HINT_ROW).contains("send"));
    assert!(!row_text(&cells, QS_FIRST_HINT_ROW).contains("type while busy to queue"));

    assert!(row_text(&cells, QS_FIRST_HINT_ROW + 1).contains("address one agent"));
    assert!(!row_text(&cells, QS_FIRST_HINT_ROW + 1).contains("plain text runs a swarm"));

    assert!(row_text(&cells, QS_FIRST_HINT_ROW + 2).contains("interrupt a running turn"));
    assert!(!row_text(&cells, QS_FIRST_HINT_ROW + 2).contains("idle: close pane"));

    assert!(row_text(&cells, QS_FIRST_HINT_ROW + 3).contains("compact transcript"));
    assert!(!row_text(&cells, QS_FIRST_HINT_ROW + 3).contains("Ctrl+Shift+M raw text"));

    assert!(row_text(&cells, QS_FIRST_HINT_ROW + 4).contains("command palette"));

    // Verify no " · " (middle dot) separator appears in any hint row.
    for i in 0..5 {
        let r = QS_FIRST_HINT_ROW + i as u16;
        let row_str = row_text(&cells, r);
        assert!(
            !row_str.contains(" \u{b7} "),
            "Row {r} should not contain ' · ' separator, got: {row_str}"
        );
    }
}

#[test]
fn quick_start_absent_on_short_pane_existing_content_unchanged() {
    let a = agents(&[("planner", "planning")]);
    // max_row = 15 leaves only 6 free rows after row 9 (needs 7: blank + rule + 5).
    let cells = empty_cells(80, 15, 2, true, &a);
    assert!(row_text(&cells, 3).contains("Your crew is ready"));
    assert!(row_text(&cells, 5).contains("planner \u{2014} planning"));
    assert!(!cells.iter().any(|c| c.row >= QS_RULE_ROW));
}

#[test]
fn quick_start_drops_below_min_width() {
    let a = agents(&[("planner", "planning")]);
    let cells = empty_cells(QS_BARE_WIDTH - 1, 30, 2, true, &a);
    assert!(!cells.iter().any(|c| c.row >= QS_RULE_ROW));
}

#[test]
fn quick_start_present_at_min_width() {
    let a = agents(&[("planner", "planning")]);
    let cells = empty_cells(QS_FULL_WIDTH, 30, 2, true, &a);
    assert!(row_text(&cells, QS_RULE_ROW).contains("quick start"));
}

#[test]
fn quick_start_absent_when_disconnected() {
    let cells = empty_cells(80, 30, 2, false, &[]);
    // Disconnected renders only the single "connecting..." row (row 3); no
    // rule/hint rows follow it regardless of how tall/wide the pane is.
    assert!(cells.iter().all(|c| c.row == 3));
}
