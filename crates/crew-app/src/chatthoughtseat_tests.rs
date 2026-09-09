use super::*;
use crate::chatthought::Thoughts;

#[test]
fn placement_seats_a_live_thought_above_the_streaming_card_and_a_settled_one_above_its_reply() {
    let mut t = Thoughts::default();
    t.absorb("coder", "x", 1_000);
    t.settle("coder", "500", 1_100);
    t.absorb("scout", "y", 2_000);
    let view = View {
        thoughts: &t,
        streaming_from: 1, // `reply` settled, `streaming` still arriving
        ..Default::default()
    };
    let mut reply = crate::chatmsgs::tests::msg("coder \u{2192} user", "done");
    reply.ts = "500".into();
    let streaming = crate::chatmsgs::tests::msg("scout", "part");
    assert_eq!(above_of(view, &reply, false), Some(Seat::Settled(0)));
    assert_eq!(
        above_of(view, &reply, true),
        None,
        "a settled block never seats on a live card"
    );
    assert_eq!(above_of(view, &streaming, true), Some(Seat::Live(0)));
    assert!(
        orphan_seats(view, &[&reply, &streaming]).is_empty(),
        "everything is seated"
    );
    assert_eq!(
        orphan_seats(view, &[&reply]),
        vec![Seat::Live(0)],
        "no card of scout's: its thought stands at the end"
    );
}
