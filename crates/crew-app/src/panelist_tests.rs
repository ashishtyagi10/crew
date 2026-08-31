use super::*;

fn row(index: usize, title: &str, focused: bool, activity: bool) -> PaneRow {
    PaneRow {
        index,
        title: title.into(),
        focused,
        activity,
        minimized: false,
        attention: None,
        busy: false,
        hovered: false,
        unread: 0,
    }
}

/// The count appears in the sidebar too — the one view that lists panes
/// you cannot see — and never on the row you are looking at.
#[test]
fn an_unread_count_rides_the_row_of_a_pane_you_are_not_in() {
    let _g = crate::app::theme_test_guard();
    let quiet = row(1, "sh", false, false);
    let loud = PaneRow {
        unread: 12,
        ..row(2, "sh", false, false)
    };
    let focused = PaneRow {
        unread: 12,
        ..row(3, "sh", true, false)
    };
    let text = |p: &PaneRow| -> String {
        let cells = cells_of(std::slice::from_ref(p), 30, 4);
        let mut v: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == R0).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(text(&loud).contains("12"), "{:?}", text(&loud));
    assert!(!text(&quiet).contains("12"));
    assert!(
        !text(&focused).contains("12"),
        "the pane you are in cannot have unread lines"
    );
}

/// A long title gives way to the count rather than overprinting it.
#[test]
fn the_title_stops_short_of_the_count() {
    let _g = crate::app::theme_test_guard();
    let long = PaneRow {
        unread: 7,
        ..row(1, "a-very-long-pane-title-indeed", false, false)
    };
    let cells = cells_of(std::slice::from_ref(&long), 30, 4);
    let at = |col: u16| cells.iter().filter(|c| c.row == R0 && c.col == col).count();
    assert!(at(26) <= 1, "two glyphs share a cell on the count's row");
    let digit = cells
        .iter()
        .find(|c| c.row == R0 && c.c == '7')
        .expect("the count was pushed off the row");
    assert!(digit.col >= 26, "the count moved out of its slot");
}

/// The same sweep the pane card's top border gets: a sidebar row carries
/// an index, a focus marker, a title, a `[+]` restore button, an unread
/// count and a status dot, added over several releases and all placed by
/// hand against `cols`. Each has to be drawn whole or not at all — `write`
/// overwrites, so a collision is a fragment rather than a doubled cell.
#[test]
fn nothing_in_a_sidebar_row_is_drawn_half_over() {
    let _g = crate::app::theme_test_guard();
    let row = PaneRow {
        index: 12,
        title: "crew \u{b7} claude".into(),
        focused: false,
        activity: true,
        minimized: true,
        attention: None,
        busy: false,
        hovered: false,
        unread: 128,
    };
    for cols in 12..=60u16 {
        let cells = cells_of(std::slice::from_ref(&row), cols, 4);
        assert!(
            cells.iter().all(|c| c.col < cols),
            "{cols}: a cell escaped the row"
        );
        let mut line: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == 2).collect();
        line.sort_by_key(|c| c.col);
        let text: String = line.iter().map(|c| c.c).collect();
        for tok in ["99+", "[+]"] {
            let head: String = tok.chars().take(tok.chars().count() - 1).collect();
            assert!(
                text.contains(tok) || !text.contains(&head),
                "{cols}: `{tok}` drawn as `{head}` \u{2014} {text:?}"
            );
        }
        // Two glyphs may not share a column either — `write` is not the
        // only painter here, and the title is placed by width.
        let mut cols_used: Vec<u16> = line.iter().map(|c| c.col).collect();
        let before = cols_used.len();
        cols_used.dedup();
        assert_eq!(cols_used.len(), before, "{cols}: two glyphs in one cell");
    }
}

/// `pane_cells` with a fixed spinner glyph.
fn cells_of(panes: &[PaneRow], cols: u16, limit: usize) -> Vec<crew_render::CellView> {
    pane_cells(panes, cols, limit, '⠋')
}

/// First row of the pane list: under the header and the mix block.
const R0: u16 = 1;

/// The row under the pointer must look different from the quiet rows
/// around it — the whole row focuses (and restores) its pane on a click,
/// and until now nothing on screen said so.
#[test]
fn a_hovered_row_lifts_its_ink_out_of_the_muted_grey() {
    let _g = crate::app::theme_test_guard();
    let quiet = [row(1, "build", false, false)];
    let hot = [PaneRow {
        hovered: true,
        ..row(1, "build", false, false)
    }];
    let ink_of = |rows: &[PaneRow]| -> Vec<(u8, u8, u8)> {
        cells_of(rows, 24, 10)
            .iter()
            .filter(|c| c.row == R0)
            .map(|c| c.fg)
            .collect()
    };
    let (a, b) = (ink_of(&quiet), ink_of(&hot));
    assert_eq!(a.len(), b.len(), "hover must not change what is drawn");
    assert_ne!(a, b, "hover must change how it is drawn");
    // Specifically: up to the theme's full-contrast ink, never a wash.
    let t = crew_theme::theme();
    assert!(b.contains(&t.ink), "hovered title reaches the ink");
    assert!(!a.contains(&t.ink), "a quiet title does not");
}

#[test]
fn pane_cells_marks_minimized_panes_with_a_restore_button() {
    let panes = [
        row(1, "build", true, false),
        PaneRow {
            minimized: true,
            ..row(2, "server", false, false)
        },
    ];
    let cells = cells_of(&panes, 24, 10);
    // The minimized pane's row carries a right-aligned [+] restore button
    // ending one cell left of the activity-dot slot: cols 18..=20. Pane
    // rows start under the header and the crew mix.
    let at = |col: u16, row: u16| {
        cells
            .iter()
            .find(|c| c.row == row && c.col == col)
            .map(|c| c.c)
    };
    assert_eq!(at(18, R0 + 1), Some('['));
    assert_eq!(at(19, R0 + 1), Some('+'));
    assert_eq!(at(20, R0 + 1), Some(']'));
    // …and only on minimized rows.
    assert!(!cells.iter().any(|c| c.c == '+' && c.row == R0));
}

#[test]
fn pane_cells_lists_focus_and_activity() {
    let _g = crate::app::theme_test_guard();
    let panes = [row(1, "build", true, false), row(2, "server", false, true)];
    let cells = cells_of(&panes, 24, 10);
    // PANES rule on row 0
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    assert!(cells.iter().any(|c| c.c == 'P' && c.row == 0));
    // focus marker + title for the focused pane on the first list row
    assert!(cells.iter().any(|c| c.c == '▸' && c.row == R0));
    assert!(cells
        .iter()
        .any(|c| c.c == 'b' && c.row == R0 && c.fg == crew_theme::theme().ink));
    // the unfocused pane's title is dimmed on the next row, with a dot
    assert!(cells
        .iter()
        .any(|c| c.c == 's' && c.row == R0 + 1 && c.fg == crew_theme::theme().text_muted));
    assert!(cells
        .iter()
        .any(|c| c.c == '●' && c.row == R0 + 1 && c.fg == crew_theme::theme().activity));
}

#[test]
fn busy_row_spins_in_the_accent_color_and_attention_still_wins() {
    // Reads the process-wide accent, so it is serialised against the
    // tests that set one (`palette`'s own floor checks walk every theme's
    // default through `set_accent`).
    let _a = crate::palette::test_guard();
    let mut busy = row(1, "swarm", false, true);
    busy.busy = true;
    let cells = cells_of(&[busy], 24, 10);
    // The spinner glyph owns the dot slot, accent-colored; the quiet
    // activity dot yields to it.
    assert!(cells
        .iter()
        .any(|c| c.c == '⠋' && c.row == R0 && c.col == 22 && c.fg == accent()));
    assert!(!cells.iter().any(|c| c.c == '●' && c.row == R0));
    // Attention beats the spinner: the needs-you marker is the loudest.
    let mut both = row(1, "swarm", false, false);
    both.busy = true;
    both.attention = Some(('!', true));
    let cells = cells_of(&[both], 24, 10);
    assert!(cells
        .iter()
        .any(|c| c.c == '!' && c.row == R0 && c.col == 22));
    assert!(!cells.iter().any(|c| c.c == '⠋'));
}

/// The list starts on the row under the rule, and the working / waiting /
/// idle tally that used to hold three rows there is gone — the rows below
/// already say which pane is which.
#[test]
fn the_list_starts_directly_under_the_header() {
    let _g = crate::app::theme_test_guard();
    let cells = cells_of(&[row(1, "solo", false, false)], 24, 10);
    let text_on = |r: u16| -> String {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == r).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(R0, 1, "no block between the rule and the first pane");
    assert!(text_on(1).contains("solo"), "{:?}", text_on(1));
    let all: String = cells.iter().map(|c| c.c).collect();
    for word in ["working", "waiting", "idle"] {
        assert!(!all.contains(word), "the tally is gone: {word}");
    }
    // The crew size still rides the rule.
    assert!(text_on(0).contains("PANES") && text_on(0).contains('1'));
}

#[test]
fn attention_row_draws_the_marker_and_tints_the_title() {
    let _g = crate::app::theme_test_guard();
    let panes = [
        row(1, "build", true, false),
        PaneRow {
            attention: Some(('!', true)),
            ..row(2, "server", false, true)
        },
    ];
    let cells = cells_of(&panes, 24, 10);
    let bell = crew_theme::theme().bell;
    // marker glyph in the dot slot, in the bell (needs-you) colour
    assert!(cells
        .iter()
        .any(|c| c.c == '!' && c.row == R0 + 1 && c.col == 22 && c.fg == bell));
    // the title is tinted too, so the row is findable at a glance
    assert!(cells
        .iter()
        .any(|c| c.c == 's' && c.row == R0 + 1 && c.fg == bell));
    // attention supersedes the quiet activity dot
    assert!(!cells.iter().any(|c| c.c == '●' && c.row == R0 + 1));
}

#[test]
fn attention_blink_off_phase_hides_the_marker_but_keeps_the_tint() {
    let _g = crate::app::theme_test_guard();
    let panes = [PaneRow {
        attention: Some(('!', false)),
        ..row(1, "server", false, false)
    }];
    let cells = cells_of(&panes, 24, 10);
    let bell = crew_theme::theme().bell;
    assert!(!cells.iter().any(|c| c.c == '!' && c.row == R0));
    assert!(cells
        .iter()
        .any(|c| c.c == 's' && c.row == R0 && c.fg == bell));
}

#[test]
fn pane_cells_respects_limit() {
    let panes: Vec<PaneRow> = (1..=5).map(|i| row(i, "x", false, false)).collect();
    let cells = cells_of(&panes, 24, 2);
    // only two pane rows are drawn; nothing reaches the row below them
    assert!(!cells.iter().any(|c| c.row == R0 + 2));
}
