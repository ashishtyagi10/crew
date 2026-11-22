//! What each section draws.
use super::*;

fn input<'a>(list: &'a crate::tabnav::NavList) -> PanelInput<'a> {
    PanelInput {
        panes: list,
        log: &[],
        log_back: 0,
        glance: None,
        strip: None,
    }
}

fn text(cells: &[CellView]) -> String {
    let mut v: Vec<&CellView> = cells.iter().collect();
    v.sort_by_key(|c| (c.row, c.col));
    v.iter().map(|c| c.c).collect()
}

/// The System section is the machine and nothing else: no pane list, no LOG.
#[test]
fn system_draws_the_machine_from_the_top() {
    let _g = crate::app::theme_test_guard();
    let s = StatsPane::new();
    let list = crate::tabnav::NavList::from(Vec::new());
    let l = NavLayout::default();
    let cells = s.panel_cells(NavSection::System, &l, 30, 60, &input(&list));
    assert!(
        cells.iter().any(|c| c.row == 0),
        "the clock is on the first row"
    );
    let t = text(&cells);
    assert!(t.contains("SYSTEM") || t.contains("LOAD"), "{t:?}");
    assert!(!t.contains("PANES") && !t.contains("LOG"), "{t:?}");
}

/// Every section stays inside the panel it is given.
#[test]
fn no_section_draws_outside_its_panel() {
    let _g = crate::app::theme_test_guard();
    let s = StatsPane::new();
    let list = crate::tabnav::NavList::from(Vec::new());
    for (cols, rows) in [(20u16, 10u16), (30, 40)] {
        for sec in NavSection::ALL {
            let l = crate::navsectionlayout::layout_for(
                sec,
                rows,
                crate::navlayout::Tail::Log(0),
                0,
                0,
            );
            for c in s.panel_cells(sec, &l, cols, rows, &input(&list)) {
                assert!(
                    c.col < cols && c.row < rows,
                    "{sec:?} {cols}x{rows}: {:?} at {},{}",
                    c.c,
                    c.col,
                    c.row
                );
            }
        }
    }
}
