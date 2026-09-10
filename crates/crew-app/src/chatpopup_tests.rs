use crate::chat::ChatPane;
use crate::chatkeys::ChatInput;
use crate::chatmention::{MentionEntry, MentionState};
use crate::popupplace::Popup;

const COLS: u16 = 100;
const ROWS: u16 = 40;

fn legend(p: &Popup) -> String {
    p.cells
        .iter()
        .filter(|c| c.row == 0 && c.c != '─')
        .map(|c| c.c)
        .collect()
}

/// A pane with a four-row slash palette open: built directly, so the
/// geometry under test does not depend on which commands a pane lists.
fn palette_pane() -> ChatPane {
    let mut p = crate::composershot_tests::pane();
    p.input = "/s".into();
    let item = |l: &str| crate::suggest::MenuItem {
        label: l.into(),
        desc: "a command".into(),
        fill: l.into(),
        ..Default::default()
    };
    p.palette = Some(crate::chatpalette::PaletteState {
        kind: crate::chatpalette::Kind::Slash,
        items: vec![
            item("/settings"),
            item("/smith"),
            item("/stop"),
            item("/smooth"),
        ],
        sel: 0,
        entries: vec![],
        touched: false,
    });
    p
}

/// The pane cell row of the pop-up's first list row.
fn first_row(p: &ChatPane) -> u16 {
    let pop = p.popup(COLS).expect("a pop-up is open");
    let g = crate::chatplace::grants(p, COLS, ROWS);
    ROWS - g.bottom - pop.rows + 1
}

/// The key prompt outranks the palette, as its keys do.
#[test]
fn the_key_prompt_outranks_the_palette_as_its_keys_do() {
    let mut p = palette_pane();
    assert!(legend(&p.popup(COLS).unwrap()).contains("commands"));
    p.keyentry = Some(crate::keyentry::KeyEntry::new("X_KEY".into()));
    assert!(legend(&p.popup(COLS).unwrap()).contains("paste X_KEY"));
    assert!(
        p.popup_item_at(COLS, ROWS, first_row(&p), 3).is_none(),
        "a field has no rows"
    );
}

/// A cell on a drawn list row resolves to that item; the frame, the margin
/// column and the pane beside the card resolve to nothing.
#[test]
fn an_item_is_found_under_its_drawn_row_and_nowhere_else() {
    let p = palette_pane();
    let pop = p.popup(COLS).unwrap();
    let top = first_row(&p) - 1;
    assert_eq!(p.popup_item_at(COLS, ROWS, top + 1, 3), Some(0));
    assert_eq!(p.popup_item_at(COLS, ROWS, top + 2, pop.cols - 2), Some(1));
    assert_eq!(p.popup_item_at(COLS, ROWS, top, 3), None, "the top border");
    assert_eq!(
        p.popup_item_at(COLS, ROWS, top + 1, 0),
        None,
        "the left border"
    );
    assert_eq!(
        p.popup_item_at(COLS, ROWS, top + 1, pop.cols - 1),
        None,
        "the right border"
    );
    assert_eq!(
        p.popup_item_at(COLS, ROWS, top + 1, pop.cols + 4),
        None,
        "beside the card"
    );
    assert_eq!(
        p.popup_item_at(COLS, ROWS, top + pop.rows - 1, 3),
        None,
        "the bottom border"
    );
}

/// A list scrolled to keep a deep selection visible maps its first drawn
/// row to the first item the scroll shows, not to item 0.
#[test]
fn a_scrolled_list_maps_rows_through_its_offset() {
    let mut p = crate::composershot_tests::pane();
    p.input = "/".into();
    crate::chatpalette::after_edit(&mut p.palette, "/", None, Vec::new);
    let n = p.palette.as_ref().unwrap().items.len();
    assert!(n > 12, "{n} commands");
    p.palette.as_mut().unwrap().sel = 12;
    let off = crate::cmdmenu::offset(n, 12);
    assert_eq!(off, 3);
    assert_eq!(p.popup_item_at(COLS, ROWS, first_row(&p), 3), Some(off));
}

/// Hover moves the selection to the row under the pointer — and not onto a
/// section title, which is not a choice.
#[test]
fn hover_moves_the_selection_and_skips_titles() {
    let mut p = palette_pane();
    let row = first_row(&p);
    {
        let pal = p.palette.as_mut().unwrap();
        pal.items[0].header = true;
        pal.sel = 1;
    }
    assert!(
        !p.popup_hover_at(COLS, ROWS, Some((row, 3))),
        "a title is not selected"
    );
    assert_eq!(p.palette.as_ref().unwrap().sel, 1);
    assert!(p.popup_hover_at(COLS, ROWS, Some((row + 2, 3))));
    assert_eq!(p.palette.as_ref().unwrap().sel, 2);
    assert!(p.palette.as_ref().unwrap().touched);
    assert!(
        !p.popup_hover_at(COLS, ROWS, Some((row + 2, 3))),
        "already there: no repaint"
    );
    assert!(!p.popup_hover_at(COLS, ROWS, None));
}

/// A click on a row picks it exactly as Enter on it would: the attach
/// picker fills the token and closes.
#[test]
fn a_click_picks_the_row_the_keyboard_would() {
    let agent = |n: &str| MentionEntry::Agent {
        name: n.into(),
        role: "a role".into(),
    };
    let mut p = crate::composershot_tests::pane();
    p.input = "@s".into();
    p.mention = Some(MentionState {
        entries: vec![],
        matches: vec![agent("smith"), agent("scout")],
        sel: 0,
    });
    let row = first_row(&p);
    let cwd = std::env::temp_dir();
    assert!(
        p.popup_click_at(COLS, ROWS, row - 1, 20, &cwd).is_none(),
        "the frame is not a row"
    );
    assert!(p.popup_click_at(COLS, ROWS, row + 1, 3, &cwd).is_some());
    assert_eq!(p.input, "@scout ");
    assert!(p.mention.is_none(), "picked, so closed");
    // The same pick by key, from the same state, lands the same way.
    let mut q = crate::composershot_tests::pane();
    q.input = "@s".into();
    q.mention = Some(MentionState {
        entries: vec![],
        matches: vec![agent("smith"), agent("scout")],
        sel: 1,
    });
    q.on_input(ChatInput::Enter, &cwd);
    assert_eq!(q.input, p.input);
}
