use super::*;
use crate::chatpalette;
use crate::grid::compose_grid;
use crate::grid::GridLayout;

#[test]
fn zoomed_hit_rects_are_the_one_drawn_tile_over_the_full_content() {
    // Regression: zoom draws the focused pane over the whole content area,
    // but hit rects used the grid placement — so wheel scrolls over a
    // zoomed pane (e.g. the /md viewer) routed to invisible grid tiles.
    let content = Rect {
        x: 10.0,
        y: 5.0,
        w: 800.0,
        h: 600.0,
    };
    let mut grid = GridLayout::new();
    for i in 0..3 {
        grid.add(i);
    }
    let placed = compose_grid(content, &grid, 16.0, GAP);
    let hits = frame_hit_rects(true, 1, 3, content, placed);
    let drawn = pane_rects_at(1, content.x, content.y, content.w, content.h, GAP)[0];
    assert_eq!(hits, vec![(1, drawn)]);
}

#[test]
fn zoomed_hit_rects_clamp_a_stale_focus_index() {
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let mut grid = GridLayout::new();
    grid.add(0);
    grid.add(1);
    let placed = compose_grid(content, &grid, 16.0, GAP);
    let hits = frame_hit_rects(true, 9, 2, content, placed);
    assert_eq!(hits[0].0, 1, "focus past the end clamps like build_frame");
}

#[test]
fn grid_hit_rects_cover_full_tiles_and_strip_thumbnails() {
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let mut grid = GridLayout::new();
    for i in 0..8 {
        grid.add(i); // 6 full tiles + 2 minimized thumbnails
    }
    let placed = compose_grid(content, &grid, 16.0, GAP);
    let hits = frame_hit_rects(false, 0, 8, content, placed);
    assert_eq!(hits.len(), 8, "every pane keeps a hit rect in grid view");
}

fn agents() -> Vec<crew_plugin::AgentInfo> {
    vec![crew_plugin::AgentInfo {
        name: "coder".into(),
        role: "codes".into(),
        model: String::new(),
    }]
}

fn legend(cells: &[crew_render::CellView]) -> String {
    let mut row0: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
    row0.sort_by_key(|c| c.col);
    row0.iter().map(|c| c.c).collect()
}

#[test]
fn palette_card_title_matches_kind() {
    assert_eq!(palette_card_title(chatpalette::Kind::Slash), "commands");
    assert_eq!(palette_card_title(chatpalette::Kind::Agent), "agents");
}

#[test]
fn slash_palette_card_shows_commands_legend_and_construct_row() {
    let mut palette = None;
    chatpalette::after_edit(&mut palette, "/mo", &[]);
    let p = palette.unwrap();
    let cells = crate::cmdmenu::menu_card(
        palette_card_title(p.kind),
        &p.items,
        p.sel,
        40,
        crate::cmdmenu::menu_rows(p.items.len()),
    );
    assert!(legend(&cells).contains("commands"));
    assert!(cells.iter().any(|c| c.c == '/'));
}

#[test]
fn agent_palette_card_shows_agents_legend_and_name_row() {
    let mut palette = None;
    chatpalette::after_edit(&mut palette, "@", &agents());
    let p = palette.unwrap();
    let cells = crate::cmdmenu::menu_card(
        palette_card_title(p.kind),
        &p.items,
        p.sel,
        40,
        crate::cmdmenu::menu_rows(p.items.len()),
    );
    assert!(legend(&cells).contains("agents"));
    assert!(cells.iter().any(|c| c.c == 'c')); // "coder" row text
}
