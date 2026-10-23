use super::backdrop;
use crate::cellgrid::CellView;

fn cell(c: char, col: u16, row: u16) -> CellView {
    CellView {
        c,
        col,
        row,
        ..CellView::default()
    }
}

/// A 20×5 pop-up: a rounded frame with ` e c ` pushed over its bottom rule
/// at 13..18, its spaces on top of the rule, as `/keys` puts its hint.
fn popup(hint: &str) -> Vec<CellView> {
    let mut cells = Vec::new();
    for col in 0..20u16 {
        let (top, bottom) = match col {
            0 => ('\u{256d}', '\u{2570}'),
            19 => ('\u{256e}', '\u{256f}'),
            _ => ('\u{2500}', '\u{2500}'),
        };
        cells.push(cell(top, col, 0));
        cells.push(cell(bottom, col, 4));
    }
    for (i, c) in hint.chars().enumerate() {
        cells.push(cell(c, 13 + i as u16, 4));
    }
    for row in 1..4u16 {
        cells.push(cell('\u{2502}', 0, row));
        cells.push(cell('\u{2502}', 19, row));
    }
    cells
}

fn quads(cells: &[CellView]) -> Vec<crate::quads::Quad> {
    let corners = crate::corners::find(cells, 10.0, 20.0);
    assert!(!corners.is_empty(), "the frame rounds");
    let geometry = (0.0, 0.0, 200.0, 100.0);
    backdrop(geometry, &corners, (10.0, 20.0), (5.0, 10.0), 6.0, [1.0; 4])
}

/// A pop-up is one box, whatever its rule says: the patch behind the words
/// on it is the shader's, feathered (`glass.wgsl`), where a quad a cell high
/// was a hard tab hanging off the card.
#[test]
fn a_pop_up_is_one_box_whatever_its_rule_says() {
    let q = quads(&popup(" e c "));
    assert_eq!(q.len(), 1);
    assert!(
        q[0].y + q[0].h <= 4.0 * 20.0 + 10.0 + 1e-3,
        "the box stops at the rule"
    );
}

/// A rule with nothing written on it adds nothing.
#[test]
fn a_bare_frame_is_one_box() {
    assert_eq!(quads(&popup("")).len(), 1);
}
