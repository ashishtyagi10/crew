/// A levelled card's interior drops to the first content row of the pane
/// beside it — its rect's top plus half its sub-cell remainder — so the
/// rail's rows sit level with the panes' (glass survey G3). The frame does
/// not move; with no pane beside it nothing does.
#[test]
fn a_levelled_interior_sits_on_the_panes_rows() {
    use crate::layout::Rect;
    let card = Rect {
        x: 4.0,
        y: 4.0,
        w: 56.0,
        h: 712.0,
    };
    let pane = Rect {
        x: 72.0,
        y: 4.0,
        w: 590.0,
        h: 668.0,
    };
    let (cw, ch) = (8.0, 16.0);
    let dy = super::level_with(card, Some(pane), cw, ch);
    assert_eq!(dy, 6.0, "668 px is 41 rows and 12 px: content 6 px down");
    assert_eq!(super::level_with(card, None, cw, ch), 0.0);
    let empty = |_: u16, _: u16| (Vec::new(), Vec::new());
    let (mut plain, mut level) = (Vec::new(), Vec::new());
    crate::panelcard::push_card_art(&mut plain, card, cw, ch, "x", (9, 9, 9), empty);
    super::push_card_art_level(&mut level, card, (cw, ch), ("x", (9, 9, 9)), dy, empty);
    assert_eq!(level[0].y, plain[0].y + dy);
    assert_eq!(level[0].y + level[0].h, plain[0].y + plain[0].h);
    assert_eq!(
        (level[1].y, level[1].h),
        (plain[1].y, plain[1].h),
        "the frame stays"
    );
}
