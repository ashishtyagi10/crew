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
fn header_shows_title_channel_and_dot() {
    let cells = header_cells(60, "general", true, false, None, 0, false);
    let line = text(&cells, 0);
    assert!(line.contains("crew"), "title missing: {line}");
    assert!(line.contains("general"), "channel missing: {line}");
    assert!(line.contains('\u{25cf}'), "connected dot missing: {line}");
}

#[test]
fn reductionist_header_has_no_msg_or_turn_chatter() {
    // A wide, idle, connected header is just: title · dot. No message count,
    // no turn count, no last-turn duration.
    let line = text(&header_cells(60, "general", true, false, None, 0, false), 0);
    assert!(
        !line.contains("msg"),
        "message counter must be gone: {line}"
    );
    assert!(!line.contains("turn"), "turn counter must be gone: {line}");
    assert!(
        !line.contains("tok"),
        "no token meter at zero spend: {line}"
    );
}

#[test]
fn connecting_dot_when_disconnected() {
    let line = text(&header_cells(60, "", false, false, None, 0, false), 0);
    assert!(line.contains('\u{25cb}'), "connecting dot missing: {line}");
}

#[test]
fn awaiting_shows_thinking_spinner() {
    let line = text(&header_cells(60, "c", true, true, None, 0, false), 0);
    assert!(line.contains("thinking"), "spinner label missing: {line}");
}

#[test]
fn active_agent_shows_name_and_elapsed_over_plain_thinking() {
    let line = text(
        &header_cells(
            60,
            "c",
            true,
            true,
            Some(("coder", 12, (9, 9, 9))),
            0,
            false,
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
    assert!(!text(&header_cells(60, "c", true, false, None, 0, false), 0).contains("tok"));
    let line = text(&header_cells(60, "c", true, false, None, 9_500, false), 0);
    assert!(line.contains("~9.5k tok"), "meter missing: {line}");
}

#[test]
fn all_cells_stay_within_width() {
    let cells = header_cells(
        20,
        "a-very-long-channel-name",
        true,
        true,
        Some(("x", 5, (9, 9, 9))),
        12_345,
        false,
    );
    assert!(cells.iter().all(|c| c.col < 20 && c.row == 0));
}

#[test]
fn esc_interrupt_hint_appears_while_busy_on_a_wide_pane() {
    let line = text(&header_cells(60, "c", true, true, None, 0, false), 0);
    assert!(line.contains("esc interrupts"), "busy hint missing: {line}");
}

#[test]
fn esc_interrupt_hint_absent_when_idle() {
    let line = text(&header_cells(60, "c", true, false, None, 0, false), 0);
    assert!(
        !line.contains("esc interrupts"),
        "hint must not appear while idle: {line}"
    );
}

#[test]
fn esc_interrupt_hint_is_first_dropped_when_narrow() {
    let wide = text(&header_cells(60, "c", true, true, None, 0, false), 0);
    assert!(wide.contains("esc interrupts"), "hint should fit: {wide}");

    // Narrow: room for the core status (spinner + dot) but not the optional
    // hint suffix — the hint must be dropped rather than clip mid-glyph.
    let narrow = text(&header_cells(24, "c", true, true, None, 0, false), 0);
    assert!(
        !narrow.contains("esc interrupts"),
        "hint should be dropped first when narrow: {narrow}"
    );
    assert!(
        narrow.contains("thinking"),
        "core status must still render: {narrow}"
    );
}

#[test]
fn compact_chip_appears_when_compact_on_a_wide_pane() {
    let line = text(&header_cells(60, "c", true, false, None, 0, true), 0);
    assert!(line.contains("compact"), "compact chip missing: {line}");
}

#[test]
fn compact_chip_absent_when_not_compact() {
    let line = text(&header_cells(60, "c", true, false, None, 0, false), 0);
    assert!(
        !line.contains("compact"),
        "chip must not appear outside compact view: {line}"
    );
}

#[test]
fn compact_chip_is_dropped_before_the_esc_hint_when_narrow() {
    // Wide: both the compact chip and the busy hint fit alongside the rest.
    let wide = text(&header_cells(60, "c", true, true, None, 0, true), 0);
    assert!(wide.contains("compact"), "chip should fit: {wide}");
    assert!(wide.contains("esc interrupts"), "hint should fit: {wide}");

    // Mid-width: room for the hint but not both — the compact chip is the
    // first of the two dropped (it's the less essential signal).
    let mid = text(&header_cells(40, "c", true, true, None, 0, true), 0);
    assert!(
        !mid.contains("compact"),
        "chip should be dropped first when narrow: {mid}"
    );
    assert!(
        mid.contains("esc interrupts"),
        "hint must still fit once the chip is gone: {mid}"
    );

    // Narrower still: neither optional segment fits, but the core status
    // (spinner + dot) always renders.
    let narrow = text(&header_cells(20, "c", true, true, None, 0, true), 0);
    assert!(!narrow.contains("compact"), "got: {narrow}");
    assert!(!narrow.contains("esc interrupts"), "got: {narrow}");
    assert!(
        narrow.contains("thinking"),
        "core status must still render: {narrow}"
    );
}
