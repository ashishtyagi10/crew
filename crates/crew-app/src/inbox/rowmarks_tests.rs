//! The few marks a row carries besides its words: `draft`, the paperclip
//! and the star, and nothing that is not one of those.
use super::tests::{draw, text, thread};
use crate::inbox::wire::Thread;

#[test]
fn a_row_says_draft_and_attached_until_the_room_runs_out_and_nothing_else() {
    let _g = crate::app::theme_test_guard();
    let mut th = thread();
    th.draft = true;
    th.attached = true;
    let one = |th: &Thread, cols| text(&draw(th, cols, false, false), 0, cols);
    let wide = one(&th, 120);
    assert!(wide.contains("Bo Park draft 3"), "{wide:?}");
    assert!(wide.ends_with("\u{2398} \u{2605} 3h"), "{wide:?}");
    let at50 = one(&th, 50);
    assert!(
        at50.ends_with(" \u{2605} 3h") && !at50.contains('\u{2398}'),
        "the paperclip goes with the count: {at50:?}"
    );
    let c = draw(&th, 120, false, false);
    let draft = c.iter().find(|c| c.c == 'd').unwrap();
    let t = crew_theme::theme();
    assert_eq!(
        draft.fg,
        crew_theme::readable::danger(&t),
        "draft in Gmail's red"
    );
    assert!(
        c.iter().all(|c| !c.mark),
        "no chips: a row is words and three marks"
    );
    let plain = one(&thread(), 120);
    assert!(!plain.contains("draft") && !plain.contains('\u{2398}'));
}
