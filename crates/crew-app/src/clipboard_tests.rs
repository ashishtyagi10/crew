use super::{one_line, paste_into_chat, screen_text};

#[test]
fn one_line_flattens_newlines() {
    assert_eq!(one_line("a\nb\r\nc"), "a b  c");
    assert_eq!(one_line("plain"), "plain");
}

#[test]
fn multiline_keeps_newlines_and_normalizes_crlf() {
    use super::multiline;
    assert_eq!(multiline("a\r\nb\rc\nd"), "a\nb\nc\nd");
    assert_eq!(multiline("plain"), "plain");
}

#[test]
fn screen_text_trims_and_drops_blank_tail() {
    use crew_term::RenderCell;
    let c = |col, row, ch| RenderCell {
        col,
        row,
        c: ch,
        fg: (0, 0, 0),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    };
    // "hi" on row 0, "x" on row 1, row 2 blank → trailing blank dropped.
    let cells = [c(0, 0, 'h'), c(1, 0, 'i'), c(0, 1, 'x')];
    assert_eq!(screen_text(&cells, 5, 3), "hi\nx");
}

// Regression for the CRITICAL finding: with the masked key prompt open,
// Cmd+V / right-click paste must never land in the visible composer —
// `insert_paste` (the actual paste entry point) isn't reachable from a
// unit test without constructing a whole `CrewApp` + windowed pane, so
// this drives `paste_into_chat`, the routing helper both real paste
// sources (`chords.rs`'s Cmd+V and `events.rs`'s right-click) funnel
// through via `insert_paste`.
#[test]
fn paste_goes_to_an_open_key_prompt_not_the_composer() {
    let mut p = crate::chat::tests::pane();
    p.keyentry = Some(crate::keyentry::KeyEntry::new("ANTHROPIC_API_KEY".into()));
    let secret = "sk-pasted-secret";
    paste_into_chat(&mut p, &format!("{secret}\n"));

    assert!(
        p.input.is_empty(),
        "the composer must stay untouched while the prompt is open"
    );
    let masked = p
        .keyentry
        .as_ref()
        .unwrap()
        .card(60)
        .iter()
        .filter(|cell| cell.c == '•')
        .count();
    assert_eq!(
        masked,
        secret.chars().count(),
        "the pasted text (minus the trailing newline) reached the prompt's buffer"
    );
}

#[test]
fn paste_reaches_the_composer_when_no_prompt_is_open() {
    let mut p = crate::chat::tests::pane();
    paste_into_chat(&mut p, "hello\nworld");
    assert_eq!(
        p.input, "hello\nworld",
        "no prompt open: ordinary paste path"
    );
}
