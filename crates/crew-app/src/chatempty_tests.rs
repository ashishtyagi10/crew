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
        let lines = wrap_to(crew_plugin::no_provider_advice(), cols);
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
fn ready_state_is_the_hint_and_two_example_asks() {
    // Claude-Code-style: no "ready" heading, no roster dump, no keybind table —
    // the muted hint, a spacer, and two asks that show what smith decides.
    // The first agent's name seeds the @-example.
    let a = agents(&[("planner", "planning"), ("coder", "implementation")]);
    let cells = empty_cells(80, 20, 2, true, &a);
    let hint = row_text(&cells, 3);
    assert!(hint.contains("Type a task"), "hint missing: {hint}");
    assert!(hint.contains("@planner"), "@-example missing: {hint}");
    // The hint wraps to two rows at 80 columns; the spacer is row 5; the
    // examples take two more; nothing else follows.
    let all: String = (3..=4).map(|r| row_text(&cells, r) + " ").collect();
    assert!(
        all.contains("/ for commands."),
        "the sentence is whole: {all}"
    );
    assert!(
        row_text(&cells, 5).is_empty(),
        "a spacer row separates them"
    );
    let examples: String = (6..=7).map(|r| row_text(&cells, r) + " ").collect();
    assert!(examples.starts_with("Try "), "{examples}");
    assert!(examples.contains("make the tests pass"), "{examples}");
    assert!(examples.contains("draft a plan first"), "{examples}");
    assert!(
        cells.iter().all(|c| c.row <= 7),
        "the hint and the examples, nothing else",
    );
}

/// The example asks wrap to every width the advice is swept at, whole, and
/// never paint past the pane. (Under ~16 columns the wrap floor is wider
/// than the pane and every row is clipped — the sweep starts where the
/// existing wrapping test starts.)
#[test]
fn the_example_asks_fit_every_swept_width() {
    let a = agents(&[("smith", "lead")]);
    for cols in [20u16, 40, 42, 80] {
        let cells = empty_cells(cols, 40, 0, true, &a);
        assert!(
            cells.iter().all(|c| c.col < cols),
            "{cols}: painted past the pane"
        );
        let rows: Vec<String> = (1..40).map(|r| row_text(&cells, r)).collect();
        let joined = rows.join(" ");
        for phrase in [
            "make the tests pass",
            "draft a plan first",
            "until you approve.",
        ] {
            assert!(joined.contains(phrase), "{cols}: lost {phrase:?}: {rows:?}");
        }
        assert!(
            !joined.contains('\u{2026}'),
            "{cols}: cut, not wrapped: {rows:?}"
        );
    }
}

/// `Type a task … / for comm` was the whole hint on a half tile, cut by the
/// column with nothing to say so. It wraps.
#[test]
fn the_hint_wraps_instead_of_clipping() {
    let a = agents(&[("smith", "lead")]);
    let cells = empty_cells(42, 20, 0, true, &a);
    let rows: Vec<String> = (1..6).map(|r| row_text(&cells, r)).collect();
    assert!(rows[0].starts_with("Type a task"), "{rows:?}");
    assert!(
        rows[1].contains("who starts"),
        "wrapped onto a second row: {rows:?}"
    );
    assert!(rows.join(" ").contains("/ for commands."), "{rows:?}");
    assert!(cells.iter().all(|c| c.col < 42));
}

/// The block is fitted to the rows the composer and footer leave it. A blank
/// spacer is the first thing to go; a sentence that still does not fit is cut
/// where the rows end and says so.
#[test]
fn a_short_pane_drops_the_spacers_before_the_words() {
    // Nine rows asked (heading, spacer, seven lines of advice at 38 wide);
    // eight given.
    let cells = empty_cells(42, 9, 0, true, &[]);
    assert!(row_text(&cells, 1).contains("No agents"));
    assert!(
        row_text(&cells, 2).starts_with("Free to start"),
        "spacer dropped first"
    );
    let last = row_text(&cells, 8);
    assert!(
        last.contains("(/model)"),
        "every word of the advice: {last}"
    );
    assert!(!last.ends_with('\u{2026}'), "nothing was cut: {last}");
    // Two given: heading and the first line of advice, marked as cut.
    let cells = empty_cells(42, 3, 0, true, &[]);
    assert!(row_text(&cells, 1).contains("No agents"));
    let last = row_text(&cells, 2);
    assert!(
        last.starts_with("Free to start") && last.ends_with('\u{2026}'),
        "{last}"
    );
    assert!(cells.iter().all(|c| c.row < 3));
}

#[test]
fn everything_clips_to_bounds() {
    let a = agents(&[("planner", "a-very-long-role-description")]);
    let cells = empty_cells(12, 6, 2, true, &a);
    assert!(cells.iter().all(|c| c.col < 12 && c.row < 6));
}
