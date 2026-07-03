use super::*;

fn text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn header_shows_title_channel_and_count() {
    let cells = header_cells(60, "general", true, 3, false, None, (0, 0), 0);
    let line = text(&cells, 0);
    assert!(line.contains("crew"), "title missing: {line}");
    assert!(line.contains("general"), "channel missing: {line}");
    assert!(line.contains("3 msgs"), "count missing: {line}");
    assert!(line.contains('\u{25cf}'), "connected dot missing: {line}");
}

#[test]
fn singular_message_and_connecting_dot() {
    let line = text(&header_cells(60, "", false, 1, false, None, (0, 0), 0), 0);
    assert!(line.contains("1 msg") && !line.contains("1 msgs"));
    assert!(line.contains('\u{25cb}'), "connecting dot missing: {line}");
}

#[test]
fn awaiting_shows_thinking_spinner() {
    let line = text(&header_cells(60, "c", true, 0, true, None, (0, 0), 0), 0);
    assert!(line.contains("thinking"), "spinner label missing: {line}");
}

#[test]
fn active_agent_shows_name_and_elapsed_over_plain_thinking() {
    let line = text(
        &header_cells(
            60,
            "c",
            true,
            0,
            true,
            Some(("coder", 12, (9, 9, 9))),
            (0, 0),
            0,
        ),
        0,
    );
    assert!(
        line.contains("coder \u{00b7} 12s"),
        "active missing: {line}"
    );
    assert!(!line.contains("thinking"), "plain spinner leaked: {line}");
}

#[test]
fn token_meter_appears_once_spend_is_nonzero() {
    assert!(!text(&header_cells(60, "c", true, 0, false, None, (0, 0), 0), 0).contains("tok"));
    let line = text(
        &header_cells(60, "c", true, 0, false, None, (9_500, 0), 0),
        0,
    );
    assert!(line.contains("~9.5k tok"), "meter missing: {line}");
}

#[test]
fn all_cells_stay_within_width() {
    let cells = header_cells(
        20,
        "a-very-long-channel-name",
        true,
        999,
        true,
        Some(("x", 5, (9, 9, 9))),
        (12345, 42),
        13_100,
    );
    assert!(cells.iter().all(|c| c.col < 20 && c.row == 0));
}

#[test]
fn turn_counter_appears_once_a_turn_completes() {
    assert!(!text(&header_cells(60, "c", true, 0, false, None, (0, 0), 0), 0).contains("turn"));
    let line = text(&header_cells(60, "c", true, 0, false, None, (0, 1), 0), 0);
    assert!(
        line.contains("1 turn") && !line.contains("1 turns"),
        "{line}"
    );
    let line = text(&header_cells(60, "c", true, 0, false, None, (0, 3), 0), 0);
    assert!(line.contains("3 turns"), "{line}");
}

#[test]
fn turn_duration_appears_after_the_turn_count_when_known() {
    // 13,100ms -> "13.1s", rendered right after the turn count.
    let line = text(
        &header_cells(60, "c", true, 0, false, None, (0, 1), 13_100),
        0,
    );
    assert!(
        line.contains("1 turn \u{00b7} 13.1s"),
        "duration missing: {line}"
    );
}

#[test]
fn turn_duration_omitted_when_turn_ms_is_zero() {
    let line = text(&header_cells(60, "c", true, 0, false, None, (0, 1), 0), 0);
    assert!(
        line.contains("1 turn") && !line.contains("\u{00b7} 0.0s"),
        "unexpected duration: {line}"
    );
}

#[test]
fn right_side_segments_are_pipe_separated() {
    // `text()` concatenates rendered cells in column order — the inter-segment
    // gap columns carry no cell at all, so consecutive segments (and the
    // pipe glyph placed between them) sit directly adjacent with no space
    // cell of their own in the reconstructed string.
    let line = text(
        &header_cells(60, "c", true, 4, false, None, (6_500, 1), 13_100),
        0,
    );
    assert!(
        line.contains("13.1s\u{2502}~6.5k"),
        "pipe between turn and token segments missing: {line}"
    );
    assert!(
        line.contains("tok\u{2502}4 msgs"),
        "pipe between token and message segments missing: {line}"
    );
    assert!(
        !line.contains("msgs\u{2502}"),
        "no pipe before the connection dot: {line}"
    );
}
