use crate::inbox::cards::tests::reading;

#[test]
fn the_reply_box_takes_two_fifths_from_the_first_key_and_scrolls_with_a_thumb() {
    let _g = crate::app::motion_test_guard();
    let mut p = reading();
    p.press(&winit::keyboard::Key::Character("r".into()), 90, 50);
    let body = p.body_rows(90, 50);
    let empty = p.cards(90, 50).reply.unwrap().h;
    assert_eq!(empty, body * 2 / 5, "40% before a word is typed");
    p.type_in(&"a long reply that keeps going on and on ".repeat(60));
    let reply = p.cards(90, 50).reply.unwrap();
    assert_eq!(reply.h, empty, "the words do not grow it");
    let edge = reply.col + reply.w - 1;
    let heavy = p.cells(90, 50).iter().any(|c| {
        c.c == '\u{2503}' && c.col == edge && (reply.row..reply.row + reply.h).contains(&c.row)
    });
    assert!(heavy, "words past its rows put a thumb on its edge");
}
