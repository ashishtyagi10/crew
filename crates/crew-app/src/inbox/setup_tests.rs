use crate::inbox::{InboxPane, Stage};

#[test]
fn the_setup_page_is_a_card_and_says_what_the_pane_does() {
    let p = InboxPane::with(Stage::Setup { found: None });
    let cells = p.cells(80, 24);
    let text: String = cells.iter().map(|c| c.c).collect();
    assert!(text.contains("set up Gmail"), "its card's name");
    assert!(cells
        .iter()
        .any(|c| (c.row, c.col, c.c) == (0, 1, '\u{256d}')));
    assert!(text.contains("read, file and reply"), "{text}");
    assert!(
        !text.contains("read-only"),
        "it has not been read-only since v0.27.30"
    );
}

#[test]
fn a_drag_on_the_setup_page_keeps_to_its_card() {
    let p = InboxPane::with(Stage::Setup { found: None });
    let page = p.cards(80, 24).page.expect("the setup page's card");
    let inner = p
        .drag_card(80, 24, (page.col + 3, page.row + 2))
        .expect("inside it");
    assert_eq!(inner, page.inner());
    let edge = p
        .cells(80, 24)
        .into_iter()
        .find(|c| c.c == '\u{2502}')
        .expect("an edge");
    assert!(
        inner.local(edge.row, edge.col).is_none(),
        "its edge is not kept"
    );
}
