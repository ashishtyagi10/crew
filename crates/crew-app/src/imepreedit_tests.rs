//! A composition is kept until it commits, and drawn over the caret's cells.
use super::*;

fn pre(text: &str, caret: Option<usize>) -> Preedit {
    Preedit {
        text: text.into(),
        caret,
        owner: Owner::Pane(0),
    }
}

/// A row of grid cells `a`, `b`, … at columns 0..10 of row 2.
fn grid() -> Vec<CellView> {
    (0..10u16)
        .map(|col| CellView {
            col,
            row: 2,
            c: char::from(b'a' + col as u8),
            ..Default::default()
        })
        .collect()
}

fn at(cells: &[CellView], col: u16, row: u16) -> Vec<&CellView> {
    cells
        .iter()
        .filter(|c| c.col == col && c.row == row)
        .collect()
}

const INK: ((u8, u8, u8), (u8, u8, u8)) = ((200, 100, 50), (0, 0, 0));

#[test]
fn a_preedit_is_kept_and_a_commit_or_disabled_clears_it() {
    let mut s = ImeState::default();
    let p = Ime::Preedit("にほ".into(), Some((6, 6)));
    assert!(s.note(&p, Owner::Pane(0)));
    assert_eq!(s.preedit, Some(pre("にほ", Some(6))));
    assert!(
        !s.note(&p, Owner::Pane(0)),
        "the same preedit changes nothing"
    );
    assert!(s.note(&Ime::Commit("日本".into()), Owner::Pane(0)));
    assert_eq!(s.preedit, None);
    s.note(&p, Owner::Pane(0));
    assert!(s.note(&Ime::Disabled, Owner::Pane(0)));
    assert_eq!(s.preedit, None);
    s.note(&p, Owner::Pane(0));
    assert!(s.note(&Ime::Preedit(String::new(), None), Owner::Pane(0)));
    assert_eq!(s.preedit, None, "an empty preedit is none");
    s.note(&p, Owner::Pane(0));
    assert!(!s.note(&Ime::Enabled, Owner::Pane(0)));
    assert!(s.preedit.is_some(), "Enabled keeps what is held");
}

/// Focus moving to another pane keeps the IME on — no Disabled comes — so
/// the composition is dropped when its owner stops taking text.
#[test]
fn a_composition_is_dropped_when_focus_leaves_its_owner() {
    let mut s = ImeState::default();
    s.note(&Ime::Preedit("か".into(), None), Owner::Pane(0));
    assert!(!s.follow(Some(Owner::Pane(0))));
    assert!(s.preedit.is_some());
    assert!(s.follow(Some(Owner::Pane(1))));
    assert_eq!(s.preedit, None);
    s.note(&Ime::Preedit("か".into(), None), Owner::Bar);
    assert!(s.follow(None), "nothing takes text");
}

/// Each wide glyph is one cell two columns wide, underlined in the ink; the
/// grid cells under them are gone and those past them stay.
#[test]
fn wide_glyphs_take_two_columns_over_the_grid() {
    let mut cells = grid();
    overlay(&mut cells, (3, 2), 40, false, &pre("にほ", Some(6)), INK);
    let ni = at(&cells, 3, 2);
    assert_eq!(ni.len(), 1, "one cell at the caret, the grid's taken out");
    assert_eq!(ni[0].c, 'に');
    assert_eq!(ni[0].deco.line, DecoLine::Single);
    assert_eq!(ni[0].fg, INK.0);
    assert!(at(&cells, 4, 2).is_empty(), "に's second column");
    let ho = at(&cells, 5, 2);
    assert_eq!((ho.len(), ho[0].c), (1, 'ほ'));
    assert_eq!(ho[0].deco.line, DecoLine::Single);
    assert!(at(&cells, 6, 2).is_empty(), "ほ's second column");
    // The caret past the end: a blank cell carrying the beam.
    let caret = at(&cells, 7, 2);
    assert_eq!(caret.len(), 1);
    assert_eq!(caret[0].cursor.shape, CursorShape::Beam);
    assert_eq!(at(&cells, 8, 2)[0].c, 'i', "the grid past it stays");
    assert_eq!(at(&cells, 2, 2)[0].c, 'c', "and before it");
}

/// The beam sits on the glyph at the caret's byte offset, and a composition
/// with no room left is drawn further left rather than cut.
#[test]
fn the_caret_sits_inside_and_a_long_one_moves_left() {
    let mut cells = grid();
    overlay(&mut cells, (8, 2), 10, false, &pre("にほ", Some(3)), INK);
    let ni = at(&cells, 6, 2);
    assert_eq!(ni[0].c, 'に', "4 columns end at column 10");
    assert_eq!(ni[0].cursor.shape, CursorShape::None);
    assert_eq!(at(&cells, 8, 2)[0].c, 'ほ');
    assert_eq!(at(&cells, 8, 2)[0].cursor.shape, CursorShape::Beam);
    assert_eq!(at(&cells, 5, 2)[0].c, 'f');
}

/// `tail` clears the row to `end`: the bar's caret and history ghost.
#[test]
fn tail_clears_the_rest_of_the_row() {
    let mut cells = grid();
    overlay(&mut cells, (2, 2), 9, true, &pre("é", None), INK);
    assert_eq!(at(&cells, 2, 2)[0].c, 'é');
    assert!((3..9).all(|c| at(&cells, c, 2).is_empty()));
    assert_eq!(at(&cells, 9, 2)[0].c, 'j', "the border column stays");
}

/// The caret cell's rect, in whole px, from the grid's top-left cell.
#[test]
fn the_cursor_area_is_the_caret_cell() {
    assert_eq!(
        cell_rect((100.0, 50.0), (3, 2), 8.0, 17.0),
        [124, 84, 8, 17]
    );
    assert_eq!(cell_rect((0.5, 0.0), (0, 0), 9.6, 20.4), [1, 0, 10, 20]);
}
