use winit::keyboard::NamedKey;

use crate::inbox::compose::On;
use crate::inbox::composekeys::tests::{on, press, reading, typed};
use crate::inbox::InboxPane;

/// Row `r` of a pane `cols` by `rows`, as text.
fn row(p: &InboxPane, cols: u16, rows: u16, r: u16) -> String {
    let mut line = vec![' '; usize::from(cols)];
    for c in p.cells(cols, rows).into_iter().filter(|c| c.row == r) {
        line[usize::from(c.col)] = c.c;
    }
    line.into_iter().collect()
}

#[test]
fn a_short_pane_keeps_a_row_for_the_words_and_the_caret_off_the_edge() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    typed(&mut p, "Yes");
    for (cols, rows) in [(40u16, 14u16), (40, 12), (60, 18)] {
        let b = p.cards(cols, rows).reply.expect("the box");
        let mail = p.cards(cols, rows).mail.unwrap();
        assert!(mail.h > b.h, "{cols}x{rows}: the mail is taller");
        let inside: Vec<String> = (b.row + 1..b.row + b.h - 1)
            .map(|r| row(&p, cols, rows, r))
            .collect();
        assert!(
            inside.iter().any(|l| l.contains("Yes\u{258f}")),
            "{cols}x{rows}: the words and the caret inside: {inside:#?}"
        );
        let edge = row(&p, cols, rows, b.row + b.h - 1);
        assert!(!edge.contains('\u{258f}'), "{cols}x{rows}: {edge}");
    }
}

#[test]
fn a_line_not_being_written_shows_its_start_and_a_hint_follows_the_caret() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    typed(&mut p, "r");
    let b = p.cards(40, 20).reply.unwrap();
    let to = row(&p, 40, 20, b.row + 1);
    assert!(to.contains("to  Ann Lee <ann@x.org>"), "{to}");
    assert!(to.contains('\u{2026}'), "cut at the edge: {to}");
    press(&mut p, NamedKey::ArrowUp);
    assert_eq!(on(&p), On::Cc);
    for _ in 0..8 {
        press(&mut p, NamedKey::Backspace);
    }
    let cc = row(&p, 40, 20, b.row + 2);
    assert!(cc.contains("cc  \u{258f}add people"), "{cc}");
}

#[test]
fn the_box_is_named_for_whom_its_to_line_says() {
    let _g = crate::app::motion_test_guard();
    let (mut p, _) = reading();
    let name = |p: &InboxPane| {
        let b = p.cards(80, 30).reply.expect("the box");
        row(p, 80, 30, b.row)
    };
    typed(&mut p, "f");
    assert!(name(&p).contains(" forward "), "{}", name(&p));
    typed(&mut p, "Kim <kim@z.org>");
    assert!(name(&p).contains(" forward to Kim "), "{}", name(&p));
    p.compose = None;
    typed(&mut p, "r");
    assert!(
        name(&p).contains(" reply to Ann Lee and bo@x.org "),
        "{}",
        name(&p)
    );
    press(&mut p, NamedKey::ArrowUp);
    press(&mut p, NamedKey::ArrowUp);
    for _ in 0..", bo@x.org".len() {
        press(&mut p, NamedKey::Backspace);
    }
    assert!(
        name(&p).contains(" reply to Ann Lee "),
        "as it is edited: {}",
        name(&p)
    );
}
