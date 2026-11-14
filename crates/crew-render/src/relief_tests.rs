use super::*;
use crate::cellgrid::CellView;

fn cell(col: u16, row: u16, c: char) -> CellView {
    CellView {
        col,
        row,
        c,
        ..Default::default()
    }
}

/// A 5x3 card: a frame with `ab` standing on its top rule, and a divider
/// rule inside it.
fn card() -> PaneScene {
    let mut cells = Vec::new();
    for (row, line) in [
        "\u{256D}ab\u{2500}\u{256E}",
        "\u{2502}x\u{2500}y\u{2502}",
        "\u{2570}\u{2500}\u{2500}\u{2500}\u{256F}",
    ]
    .iter()
    .enumerate()
    {
        for (col, c) in line.chars().enumerate() {
            cells.push(cell(col as u16, row as u16, c));
        }
    }
    PaneScene {
        cells,
        glass: true,
        ..Default::default()
    }
}

fn text(p: &PaneScene, row: u16) -> String {
    p.cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| c.c)
        .collect()
}

/// The frame goes, and only the frame: the legend on the top rule and the
/// rule inside the card are still there.
#[test]
fn an_unframed_card_keeps_its_legend_and_its_inner_rules() {
    let out = unframed(&card());
    assert_eq!(text(&out, 0), " ab  ");
    assert_eq!(text(&out, 1), " x\u{2500}y ");
    assert_eq!(text(&out, 2), "     ");
    assert_eq!(out.cells.len(), card().cells.len(), "no cell is dropped");
}

/// A relief card is square: no arc at its corners, only the shadows' blur.
#[test]
fn a_relief_card_has_square_corners() {
    assert_eq!(RADIUS, 0.0);
}

/// The quad is padded for the glass's shadows; a relief card's two must fit
/// in it too at the highest lift, each thrown its distance and a quarter
/// past its blur (the same bound `shadow_padding_covers_its_falloff` holds).
#[test]
fn the_padding_holds_both_relief_shadows_at_full_lift() {
    let src = include_str!("glass.wgsl");
    let num = |name: &str| -> f32 {
        let key = format!("const {name}: f32 = ");
        let rest = &src[src.find(&key).unwrap_or_else(|| panic!("{name} missing")) + key.len()..];
        rest[..rest.find(';').expect("unterminated")]
            .trim()
            .parse()
            .expect("non-numeric")
    };
    let k = num("NEU_REST") + num("NEU_LIFT") * num("NEU_TOP");
    let reach = k * (num("NEU_DIST") + 1.25 * num("NEU_BLUR"));
    assert!(num("PAD") >= reach, "PAD cannot hold a {reach}px shadow");
}

/// Focus is height on these themes: the card with the keys throws a shadow
/// at least twice as long as a card at rest.
#[test]
fn the_focused_card_stands_twice_as_high_as_one_at_rest() {
    let src = include_str!("glass.wgsl");
    let num = |name: &str| -> f32 {
        let key = format!("const {name}: f32 = ");
        let rest = &src[src.find(&key).unwrap_or_else(|| panic!("{name} missing")) + key.len()..];
        rest[..rest.find(';').expect("unterminated")]
            .trim()
            .parse()
            .expect("non-numeric")
    };
    assert!(num("NEU_REST") + num("NEU_LIFT") >= 2.0 * num("NEU_REST"));
}
