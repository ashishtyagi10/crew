use super::*;

/// Cells of `rows`, one char a cell, blanks kept.
fn grid(rows: &[&str]) -> Vec<CellView> {
    let mut v = Vec::new();
    for (r, line) in rows.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            v.push(CellView {
                col: c as u16,
                row: r as u16,
                c: ch,
                ..Default::default()
            });
        }
    }
    v
}

fn text(cells: &[CellView], row: u16) -> String {
    let mut r: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    r.sort_by_key(|c| c.col);
    r.iter().map(|c| c.c).collect()
}

const FORM: &[&str] = &[
    "╭ group ────────╮",
    "│ ╭ Size ─╮     │",
    "│ │14     │  x  │",
    "│ ╰───────╯     │",
    "╰───────────────╯",
    " ╭──┬──╮         ",
    " │a │b │         ",
    " ╰──┴──╯         ",
];

/// A box is read off its four corners; one that holds another is a group
/// and gets no sheet, so only the field and the two-panel box are wells.
#[test]
fn only_the_boxes_that_hold_no_box_are_wells() {
    let cells = grid(FORM);
    assert_eq!(boxes(&cells).len(), 3);
    let w = wells(&boxes(&cells));
    let field = Box4 {
        c0: 2,
        r0: 1,
        c1: 10,
        r1: 3,
    };
    let panels = Box4 {
        c0: 1,
        r0: 5,
        c1: 7,
        r1: 7,
    };
    assert_eq!(w, vec![field, panels]);
}

/// A box loses its strokes and nothing else: its legend and its value
/// stay, and so does the divider inside it. The group round the field
/// loses its rule too and keeps its legend.
#[test]
fn a_box_loses_its_strokes_and_keeps_its_words() {
    let mut cells = grid(FORM);
    let all = boxes(&cells);
    unstroke(&mut cells, &all);
    assert_eq!(text(&cells, 0), "  group          ");
    assert_eq!(text(&cells, 1), "    Size         ");
    assert_eq!(text(&cells, 2), "   14        x   ");
    assert_eq!(text(&cells, 3), "                 ");
    assert_eq!(
        text(&cells, 5),
        "                 ",
        "junctions on the edge go too"
    );
    assert_eq!(
        text(&cells, 6),
        "  a │b           ",
        "the divider inside stays"
    );
}

/// Three corners are not a box: a rule that merely turns is left alone.
#[test]
fn a_box_missing_a_corner_is_not_a_well() {
    let cells = grid(&["╭───╮", "│   │", "╰────"]);
    assert!(boxes(&cells).is_empty());
}

/// A group scrolled half out of the pane has no bottom corners: it is
/// unruled all the same, and the field inside it is still a well.
#[test]
fn a_group_cut_by_the_pane_s_edge_is_unruled_too() {
    let rows = [
        "╭ window ───╮",
        "│ ╭ Nav ─╮  │",
        "│ │210   │  │",
        "│ ╰──────╯  │",
    ];
    let cells = grid(&rows);
    assert_eq!(boxes(&cells).len(), 1, "the field alone is whole");
    let cut = cut_boxes(&cells);
    assert_eq!(
        cut,
        vec![Box4 {
            c0: 0,
            r0: 0,
            c1: 12,
            r1: 3
        }]
    );
    let mut cells = cells;
    unstroke(&mut cells, &cut);
    assert_eq!(text(&cells, 0), "  window     ");
    assert_eq!(
        text(&cells, 2),
        "  │210   │   ",
        "the field's own rules are its own"
    );
    assert!(
        cut_boxes(&grid(FORM)).is_empty(),
        "a whole box is not a cut one"
    );
}

/// The sheet covers the box's own cells, top row to bottom row, and stands
/// a quarter cell in at each side.
#[test]
fn a_well_s_sheet_covers_its_cells_and_stands_in_at_the_sides() {
    let b = Box4 {
        c0: 2,
        r0: 1,
        c1: 10,
        r1: 3,
    };
    let r = rect_of(&b, (100.0, 50.0), 8.0, 16.0);
    assert_eq!((r.x, r.y), (100.0 + 2.25 * 8.0, 50.0 + 16.0));
    assert_eq!((r.w, r.h), (8.5 * 8.0, 48.0));
}

fn pane_scenes() -> Vec<PaneScene> {
    let content = PaneScene {
        cells: grid(FORM),
        x: 10.0,
        y: 20.0,
        ..Default::default()
    };
    let card = PaneScene {
        glass: true,
        ..Default::default()
    };
    vec![content, card]
}

/// On a soft theme the pane's boxes are pressed in: one sunk sheet a well,
/// after the card's own sheet, and the strokes gone from the content.
#[test]
fn a_soft_theme_presses_a_pane_s_boxes_into_wells() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Porcelain);
    let mut scenes = pane_scenes();
    press(&mut scenes, 8.0, 16.0);
    assert_eq!(scenes.len(), 4, "the content, the card, two wells");
    assert!(scenes[2..].iter().all(|s| s.glass && s.lift < 0.0));
    assert_eq!(text(&scenes[0].cells, 3), "                 ");
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// Every other theme strokes its boxes as it always did.
#[test]
fn a_stroked_theme_keeps_its_boxes() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Gruvbox);
    let mut scenes = pane_scenes();
    press(&mut scenes, 8.0, 16.0);
    assert_eq!(scenes.len(), 2);
    assert_eq!(text(&scenes[0].cells, 1), FORM[1]);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}
