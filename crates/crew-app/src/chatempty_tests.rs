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

/// The advice comes at three lengths and the pane takes the longest that
/// fits, so what a new user reads is a whole sentence rather than a cut one.
/// A blank spacer is still the first thing to go when even that is too much.
#[test]
fn a_short_pane_takes_a_shorter_sentence_rather_than_a_cut_one() {
    let rows_of = |max_row: u16| -> Vec<String> {
        let cells = empty_cells(42, max_row, 0, true, &[]);
        (0..max_row).map(|r| row_text(&cells, r)).collect()
    };
    // Room for everything: the long form, every word of it.
    let tall = rows_of(12);
    assert!(tall[1].contains("No agents"));
    assert!(tall.join(" ").contains("(/model)"), "{tall:?}");
    // Eight rows: the long form (seven lines at 38 wide) does not fit, so a
    // shorter one arrives WHOLE — the old behaviour cut it at the row budget.
    let short = rows_of(9);
    let text = short.join(" ");
    assert!(short[1].contains("No agents"));
    assert!(text.contains("crew picks it up."), "{short:?}");
    assert!(
        !short.iter().any(|r| r.ends_with('\u{2026}')),
        "nothing was cut: {short:?}"
    );
    // Three rows: the shortest form, still whole, still naming the door.
    let tiny = rows_of(4);
    assert!(tiny.join(" ").contains("/model"), "{tiny:?}");
    assert!(
        !tiny.iter().any(|r| r.ends_with('\u{2026}')),
        "nothing was cut: {tiny:?}"
    );
}

#[test]
fn everything_clips_to_bounds() {
    let a = agents(&[("planner", "a-very-long-role-description")]);
    let cells = empty_cells(12, 6, 2, true, &a);
    assert!(cells.iter().all(|c| c.col < 12 && c.row < 6));
}
