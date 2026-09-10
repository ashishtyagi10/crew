//! A card legend that gives way to the `[-][x]` buttons marks its cut and
//! counts display columns — it was a char clip with no `…`, landing under
//! `title_budget` so the frame's own clip never fired.
use super::{pane_card, Bar};

fn bar(title: &'static str) -> Bar<'static> {
    Bar {
        index: Some(2),
        title,
        focused: true,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: true,
        focus_t: 1.0,
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        pinned: false,
        at_cmd: None,
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread: 0,
        doc: false,
    }
}

fn top_row(cells: &[crew_render::CellView]) -> String {
    let mut by_col = std::collections::BTreeMap::new();
    for c in cells.iter().filter(|c| c.row == 0) {
        by_col.insert(c.col, c.c);
    }
    by_col.values().collect()
}

#[test]
fn a_legend_that_gives_way_to_the_buttons_marks_its_cut() {
    let _g = crate::app::theme_test_guard();
    let top = top_row(&pane_card(24, 5, &bar("cargo test --workspace")));
    assert!(top.contains('\u{2026}'), "{top:?}");
    assert!(top.contains("[-]") && top.contains("[x]"), "{top:?}");
    assert!(!top.contains("workspace"), "{top:?}");
    let wide = top_row(&pane_card(60, 5, &bar("cargo test --workspace")));
    assert!(wide.contains("2 cargo test --workspace"), "{wide:?}");
}
