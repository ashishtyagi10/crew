// The card tests share the queue fixture with the queue's own tests.
use crate::layout::Rect;
use crate::toast::tests::toasts_with;
use crate::toast::{push_toasts, Toasts};

#[test]
fn card_geometry_is_cell_quantized_and_right_aligned() {
    let mut t = toasts_with(1, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 100.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    // Far past the slide (still alive): resting position.
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    assert_eq!(scenes.len(), 1);
    let s = &scenes[0];
    // "toast 0" = 7 cols + 4 frame/pad = 11 cols → 88px wide, 3 rows tall.
    assert_eq!((s.w, s.h), (88.0, 48.0));
    // Right-aligned to content minus the gap: 100 + 800 - 8 - 88.
    assert_eq!(s.x, 804.0);
    assert!(s.overlay, "toasts must ride the opaque overlay pass");
    assert!(!s.glass);
}

#[test]
fn cards_stack_downward_with_a_gap() {
    let mut t = toasts_with(2, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    assert_eq!(scenes.len(), 2);
    assert_eq!(scenes[0].y, 8.0);
    assert_eq!(scenes[1].y, 8.0 + 48.0 + 8.0);
}

/// A card that names a pane is a shortcut to it; one that names none is still
/// dismissible. Both leave the stack when clicked — a card that has been
/// answered and stays on screen says it wasn't.
#[test]
fn a_click_takes_the_card_and_reports_its_pane() {
    let mut t = Toasts::default();
    t.push_for(
        "agent-7 is waiting".into(),
        "waiting",
        true,
        0,
        Some("agent-7".into()),
    );
    t.push("copied".into(), "note", false, 0);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 1_000, None);
    let at = |i: usize| (scenes[i].x + 2.0, scenes[i].y + 2.0);

    let (x, y) = at(1);
    assert_eq!(t.pane_at(x, y), None, "a plain note names no pane");
    assert!(t.dismiss_at(x, y));

    let (x, y) = at(0);
    assert_eq!(t.pane_at(x, y), Some("agent-7"));
    assert!(t.dismiss_at(x, y));
    assert!(
        !t.dismiss_at(x, y),
        "the card is gone; the click falls through"
    );
}

/// A message longer than one row takes a second row rather than losing its
/// tail: every word of the error arrives, the card is a row taller, and the
/// stack below it moves down by that row. Three rows' worth still clips —
/// on the second row, and marked.
#[test]
fn a_long_error_wraps_to_a_second_row_and_the_stack_follows() {
    use crate::toastcard::{fit, MAX_TEXT_COLS};
    let msg = "error: failed to spawn shell: No such file or directory (os error 2)";
    let rows = fit(msg, MAX_TEXT_COLS);
    assert_eq!(rows.len(), 2, "{rows:?}");
    let joined = rows.join(" ");
    for word in msg.split(' ') {
        assert!(joined.contains(word), "{word} lost: {rows:?}");
    }
    assert!(rows.iter().all(|r| !r.ends_with('\u{2026}')), "{rows:?}");
    let more = format!("{msg} {msg}");
    let rows = fit(&more, MAX_TEXT_COLS);
    assert_eq!(rows.len(), 2);
    assert!(rows[1].ends_with('\u{2026}'), "a cut is marked: {rows:?}");

    let mut t = Toasts::default();
    t.push(msg.into(), "error", true, 1_000);
    t.push("copied".into(), "note", false, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    assert_eq!(scenes[0].h, 64.0, "four rows: border, two of text, border");
    assert_eq!(scenes[1].y, 8.0 + 64.0 + 8.0, "the next card sits under it");
    let text: String = scenes[0]
        .cells
        .iter()
        .filter(|c| c.row == 2)
        .map(|c| c.c)
        .collect();
    assert!(
        text.contains("(os error 2)"),
        "row two carries the tail: {text:?}"
    );
}
