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

/// A picker's wash meets both strokes of its card, as the inbox row's does:
/// it was only as wide as the rows' measure, so it ended two columns short
/// of the right stroke, and ten in a floored card.
#[test]
fn the_wash_meets_both_strokes() {
    let _g = crate::app::theme_test_guard();
    let wash = crew_theme::readable::selection_bg(&crew_theme::theme());
    for text in ["/d", "/theme ", "/model "] {
        let items = crate::suggest::menu_items(text);
        for cols in [crate::cmdmenu::popup_size(&items, 120).0, 60] {
            let rows = crate::cmdmenu::menu_rows(items.len());
            let cells = crate::cmdmenu::menu_card("t", &items, 1, cols, rows);
            let row = cells
                .iter()
                .find(|c| c.c == '\u{203a}')
                .expect("marked")
                .row;
            let on = cells.iter().filter(|c| c.row == row && c.bg == wash);
            let washed: Vec<u16> = on.map(|c| c.col).collect();
            assert_eq!(washed.iter().min(), Some(&1), "{text} at {cols}");
            assert_eq!(washed.iter().max(), Some(&(cols - 2)), "{text} at {cols}");
        }
    }
}
