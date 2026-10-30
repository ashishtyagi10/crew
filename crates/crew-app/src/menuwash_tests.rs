use crew_render::CellView;

use super::wash_row;

fn cell(col: u16, row: u16, c: char, bg: (u8, u8, u8), mark: bool) -> CellView {
    CellView {
        col,
        row,
        c,
        fg: crew_theme::theme().text_muted,
        bg,
        mark,
        ..Default::default()
    }
}

#[test]
fn the_selected_row_is_washed_whole_and_its_ink_reads_on_it() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let wash = crew_theme::readable::selection_bg(&t);
        let chip = (200, 10, 10);
        let mut cells = vec![
            cell(0, 1, '\u{203a}', t.page_bg, false),
            cell(2, 1, 't', t.page_bg, false),
            cell(3, 1, 'h', chip, true),
            cell(2, 0, 'x', t.page_bg, false),
        ];
        wash_row(&mut cells, 1, 8);
        let row: Vec<&CellView> = cells.iter().filter(|c| c.row == 1).collect();
        let mut cols: Vec<u16> = row.iter().map(|c| c.col).collect();
        cols.sort();
        assert_eq!(cols, (0..8).collect::<Vec<_>>(), "{id:?}: whole");
        for c in row.iter().filter(|c| !c.mark) {
            assert_eq!(c.bg, wash, "{id:?} col {}", c.col);
        }
        let hit = row.iter().find(|c| c.mark).unwrap();
        assert_eq!(hit.bg, chip, "{id:?}: the match chip keeps its own");
        let t_cell = row.iter().find(|c| c.c == 't').unwrap();
        let ratio = crew_theme::contrast_ratio(t_cell.fg, wash);
        assert!(
            ratio >= crew_theme::contrast::text_floor() - 0.01,
            "{id:?}: {ratio}"
        );
        let other = cells.iter().find(|c| c.row == 0).unwrap();
        assert_eq!(other.bg, t.page_bg, "{id:?}: the other rows stay clear");
    }
}
