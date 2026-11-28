use crate::inbox::cards::tests::reading;
use crate::inbox::composedraw::place;
use crate::inbox::composeview::first;
use crate::inbox::InboxPane;

pub(super) const COLS: u16 = 90;
pub(super) const ROWS: u16 = 50;

/// A mail long enough to scroll and a reply of 40 lines, caret at its end.
pub(super) fn writing() -> InboxPane {
    let mut p = reading();
    let long = (0..120).map(|i| format!("mail {i}\n")).collect::<String>();
    p.reading.as_mut().unwrap().mails.as_mut().unwrap()[0].text = long;
    p.press(&winit::keyboard::Key::Character("r".into()), COLS, ROWS);
    let c = p.compose.as_mut().unwrap();
    c.body.text = (0..40).map(|i| format!("line {i}\n")).collect::<String>();
    c.body.at = c.body.text.chars().count();
    p
}

pub(super) fn top(p: &InboxPane) -> usize {
    let b = p.cards(COLS, ROWS).reply.unwrap();
    let c = p.compose.as_ref().unwrap();
    let w = crate::inbox::composedraw::text_w(b.w);
    let room = usize::from(crate::inbox::composegeo::geo(c, 0, b.h, w as u16).room);
    let total = crate::inbox::draw::wrap(&c.body.text, w).len();
    first(c, total, place(&c.body.text, c.body.at, w).0, room, w)
}

pub(super) fn mail_scroll(p: &InboxPane) -> usize {
    p.reading.as_ref().unwrap().scroll
}

/// Pointer on the reply box, and on the mail's card.
pub(super) fn spots(p: &InboxPane) -> ((u16, u16), (u16, u16)) {
    let k = p.cards(COLS, ROWS);
    let (r, m) = (k.reply.unwrap(), k.mail.unwrap());
    ((r.row + 2, r.col + 5), (m.row + 2, m.col + 5))
}

/// The heavy thumb's first row on the reply's edge.
fn thumb_row(p: &InboxPane) -> u16 {
    let r = p.cards(COLS, ROWS).reply.unwrap();
    p.cells(COLS, ROWS)
        .iter()
        .filter(|c| c.c == '\u{2503}' && c.col == r.col + r.w - 1)
        .filter(|c| (r.row..r.row + r.h).contains(&c.row))
        .map(|c| c.row)
        .min()
        .expect("a thumb")
}

#[test]
fn the_wheel_over_the_reply_moves_its_view_and_not_the_mail() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let (end, mail) = (top(&p), mail_scroll(&p));
    p.scroll_wheel(3, COLS, ROWS, Some(reply));
    assert_eq!(top(&p), end - 3);
    assert_eq!(mail_scroll(&p), mail);
}

#[test]
fn the_wheel_over_the_mail_moves_the_mail_and_not_the_reply() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (_, mail) = spots(&p);
    let (view, was) = (top(&p), mail_scroll(&p));
    p.scroll_wheel(-3, COLS, ROWS, Some(mail));
    assert_ne!(mail_scroll(&p), was);
    assert_eq!(top(&p), view);
    // Unknown pointer: the mail's, as before.
    let was = mail_scroll(&p);
    p.scroll_wheel(1, COLS, ROWS, None);
    assert_ne!(mail_scroll(&p), was);
}

#[test]
fn typing_and_caret_moves_return_the_view_to_the_caret() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let end = top(&p);
    p.scroll_wheel(5, COLS, ROWS, Some(reply));
    assert_eq!(top(&p), end - 5);
    p.type_in("x");
    assert_eq!(top(&p), end, "typing follows the caret");
    p.scroll_wheel(5, COLS, ROWS, Some(reply));
    p.compose.as_mut().unwrap().body.at -= 1;
    let c = p.compose.as_ref().unwrap();
    assert!(c.view.is_some(), "the stale view is still stored");
    assert_eq!(top(&p), end, "a caret move follows the caret exactly");
}

#[test]
fn the_view_is_clamped_at_both_ends() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let end = top(&p);
    p.scroll_wheel(-500, COLS, ROWS, Some(reply));
    assert_eq!(top(&p), end, "no further than the last page");
    p.scroll_wheel(500, COLS, ROWS, Some(reply));
    assert_eq!(top(&p), 0);
}

#[test]
fn the_thumb_follows_the_offset() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let low = thumb_row(&p);
    p.scroll_wheel(500, COLS, ROWS, Some(reply));
    assert!(thumb_row(&p) < low, "the thumb rose with the words");
}

#[test]
fn a_click_while_scrolled_lands_on_the_clicked_line() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let b = p.cards(COLS, ROWS).reply.unwrap();
    let w = crate::inbox::composedraw::text_w(b.w) as u16;
    let words = crate::inbox::composegeo::geo(p.compose.as_ref().unwrap(), 0, b.h, w).words;
    for k in [0u16, 2] {
        p.scroll_wheel(7, COLS, ROWS, Some(reply));
        let shown = top(&p);
        p.compose_click(COLS, ROWS, b.row + words + k, b.col + 4);
        let c = p.compose.as_ref().unwrap();
        let line = place(&c.body.text, c.body.at, usize::from(w)).0;
        assert_eq!(line, shown + usize::from(k), "row {k} of the view");
        assert_eq!(top(&p), shown, "the view stays put under the click");
    }
}

#[test]
fn a_caret_scrolled_out_of_view_is_not_drawn() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let bar = |p: &InboxPane| p.cells(COLS, ROWS).iter().any(|c| c.c == '\u{258f}');
    assert!(bar(&p), "the caret shows at the end");
    p.scroll_wheel(500, COLS, ROWS, Some(reply));
    assert!(!bar(&p), "its row is out of view");
}

#[test]
fn a_caret_scrolled_above_the_view_is_not_drawn_either() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    p.compose.as_mut().unwrap().body.at = 0;
    p.scroll_wheel(-500, COLS, ROWS, Some(reply));
    assert!(top(&p) > 0);
    assert!(!p.cells(COLS, ROWS).iter().any(|c| c.c == '\u{258f}'));
}

#[test]
fn a_resize_that_rewraps_the_words_returns_the_view_to_the_caret() {
    let _g = crate::app::motion_test_guard();
    let mut p = writing();
    let (reply, _) = spots(&p);
    let end = top(&p);
    p.scroll_wheel(5, COLS, ROWS, Some(reply));
    assert_eq!(top(&p), end - 5);
    let b = p.cards(COLS, ROWS).reply.unwrap();
    let w = crate::inbox::composedraw::text_w(b.w);
    let c = p.compose.as_ref().unwrap();
    let room = usize::from(crate::inbox::composegeo::geo(c, 0, b.h, w as u16).room);
    let total = crate::inbox::draw::wrap(&c.body.text, w).len();
    let caret = place(&c.body.text, c.body.at, w).0;
    let follow = caret.saturating_sub(room - 1);
    assert_eq!(
        first(c, total, caret, room, w + 1),
        follow,
        "narrower or wider"
    );
    assert_eq!(
        first(c, total, caret, room + 1, w),
        caret.saturating_sub(room)
    );
}
