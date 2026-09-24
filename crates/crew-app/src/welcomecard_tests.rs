use super::*;

fn rect() -> Rect {
    Rect {
        x: 10.0,
        y: 10.0,
        w: 400.0,
        h: 300.0,
    }
}

/// The glass scene — the one the sheet, its shadow and its rim are drawn
/// under.
fn sheet(scenes: &[PaneScene]) -> &PaneScene {
    scenes.iter().find(|s| s.glass).expect("a card has a sheet")
}

/// The welcome stands in for a lone focused terminal, so its sheet rides at a
/// focused pane's rise. It shipped lit-stroked but flat (lift 0), which on a
/// sheer window read as no card at all.
#[test]
fn the_lit_card_rises_like_a_focused_pane() {
    let mut scenes = Vec::new();
    push_card_lit(&mut scenes, rect(), 8.0, 16.0, "crew", |_, _| Vec::new());
    let s = sheet(&scenes);
    assert_eq!(s.lift, crate::spotlight::lift_for(0, 0, 0, 1.0));
    assert!(s.focused);
}

/// A roomy welcome: 120×40 cells of card.
fn roomy() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 122.0 * 8.0,
        h: 42.0 * 16.0,
    }
}

/// The rain block rides its own card, above the welcome's: the user looked
/// for an edge round the rain and found glyphs printed on the page.
#[test]
fn the_rain_block_sits_on_its_own_risen_card() {
    let mut scenes = Vec::new();
    push_welcome(&mut scenes, roomy(), 8.0, 16.0, 7, Some(2));
    let sheets: Vec<_> = scenes.iter().filter(|s| s.glass).collect();
    assert_eq!(sheets.len(), 2, "the welcome card and the block's");
    let (outer, block) = (sheets[0], sheets[1]);
    assert!(block.lift > outer.lift, "the block rises above the welcome");
    assert!(block.x > outer.x && block.y > outer.y);
    assert!(block.x + block.w < outer.x + outer.w && block.y + block.h < outer.y + outer.h);
    assert!(block.cells.iter().any(|c| c.c == '╭'), "a framed card");
}

/// Every line the block holds is inside its frame — the rain, the name, the
/// tagline, the hints, the restore offer — and the version stamp is not.
#[test]
fn the_block_frames_every_line_but_the_stamp() {
    let (cols, rows) = (120, 40);
    let cells = crate::welcome::welcome_cells_animated(cols, rows, 7, Some(2));
    let (c, r, w, h) = block_rect(&cells, cols, rows, true).expect("room for a block");
    for cell in cells.iter().filter(|x| x.row + 1 < rows) {
        assert!(
            cell.col > c && cell.col < c + w - 1,
            "{:?} at ({}, {}) is inside",
            cell.c,
            cell.col,
            cell.row
        );
        assert!(
            cell.row > r && cell.row < r + h - 1,
            "{:?} at ({}, {}) is inside",
            cell.c,
            cell.col,
            cell.row
        );
    }
    assert!(r + h < rows, "the stamp row stays the card's");
}

/// The block is centred on the card, give or take the odd cell.
#[test]
fn the_block_is_centred() {
    let (cols, rows) = (120, 40);
    let cells = crate::welcome::welcome_cells_animated(cols, rows, 7, Some(2));
    let (c, r, w, h) = block_rect(&cells, cols, rows, true).unwrap();
    assert!(
        (i32::from(c) - i32::from(cols - c - w)).abs() <= 1,
        "horizontally"
    );
    assert!(
        (i32::from(r) - i32::from(rows - r - h)).abs() <= 1,
        "vertically"
    );
}

/// No room for a padded frame: no block card, and no panic.
#[test]
fn a_tight_welcome_has_no_block() {
    for (cols, rows) in [(20, 6), (40, 12), (1, 1)] {
        let cells = crate::welcome::welcome_cells_animated(cols, rows, 7, None);
        let _ = block_rect(&cells, cols, rows, false);
    }
    let mut scenes = Vec::new();
    push_welcome(&mut scenes, rect(), 8.0, 16.0, 7, None);
}
