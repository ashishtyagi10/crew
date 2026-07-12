use super::render;
use crate::farpane::FarPane;

/// Reconstruct rendered text per row (opaque blanks render as a block in some
/// paths; this pane uses `to_cells`, so blanks are simply absent).
fn text(cells: &[crew_render::CellView]) -> String {
    let max_row = cells.iter().map(|c| c.row).max().unwrap_or(0);
    let mut lines = vec![String::new(); max_row as usize + 1];
    let mut sorted: Vec<(u16, u16, char)> = cells.iter().map(|c| (c.row, c.col, c.c)).collect();
    sorted.sort_unstable();
    let mut last = (u16::MAX, 0u16);
    for (row, col, c) in sorted {
        if (row, col) != last {
            lines[row as usize].push(c);
        }
        last = (row, col);
    }
    lines.join("\n")
}

fn fixture_pane(key: &str) -> FarPane {
    let base = std::env::temp_dir().join(format!("crew_far_render_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("alpha")).unwrap();
    std::fs::write(base.join("readme.md"), b"x").unwrap();
    FarPane::new(base)
}

#[test]
fn renders_two_panels_and_function_bar() {
    let cells = render(&fixture_pane("panels"), 80, 24);
    assert!(!cells.is_empty());
    let t = text(&cells);
    // both directory entries appear (dirs get a trailing slash)
    assert!(t.contains("alpha/"), "missing dir entry; got:\n{t}");
    assert!(t.contains("readme.md"), "missing file entry");
    // the Far-style function bar
    assert!(t.contains("Quit"), "missing function bar");
    assert!(t.contains("Copy"));
    // rounded panel borders
    assert!(cells.iter().any(|c| c.c == '╭'));
}

#[test]
fn panels_share_a_single_divider() {
    let cells = render(&fixture_pane("divider"), 80, 24);
    let t = text(&cells);
    // One shared border column between the panels, joined into the frame.
    assert!(t.contains('┬'), "top junction missing:\n{t}");
    assert!(t.contains('┴'), "bottom junction missing:\n{t}");
    assert!(!t.contains("╮╭"), "unmerged panel corners:\n{t}");
    // No two vertical borders in adjacent columns anywhere (the old `││` gap).
    let mut vbars: Vec<(u16, u16)> = cells
        .iter()
        .filter(|c| c.c == '│')
        .map(|c| (c.row, c.col))
        .collect();
    vbars.sort_unstable();
    assert!(
        !vbars
            .windows(2)
            .any(|w| w[0].0 == w[1].0 && w[0].1 + 1 == w[1].1),
        "adjacent vertical borders survive:\n{t}"
    );
}

#[test]
fn function_bar_highlights_actions_far_style() {
    let cells = render(&fixture_pane("fbar"), 80, 24);
    let bar_row = cells.iter().map(|c| c.row).max().unwrap();
    let bar: Vec<_> = cells.iter().filter(|c| c.row == bar_row).collect();
    let mut v: Vec<(u16, char)> = bar.iter().map(|c| (c.col, c.c)).collect();
    v.sort_unstable();
    let s: String = v.into_iter().map(|(_, c)| c).collect();
    // Key number outside the block, a gap, then the action on a solid pill.
    assert!(s.contains("▐Help▌"), "label block caps missing: {s}");
    assert!(s.contains("F10▐Quit▌"), "F10 keeps its number: {s}");
    let f = bar.iter().find(|c| c.c == 'F').unwrap();
    let h = bar.iter().find(|c| c.c == 'H').unwrap();
    assert_eq!(h.bg, f.fg, "label must sit on an accent block");
    assert_ne!(h.bg, h.fg, "label text must contrast with its block");
}

#[test]
fn tiny_renders_nothing() {
    assert!(render(&fixture_pane("tiny"), 8, 2).is_empty());
}

#[test]
fn legend_shows_the_entry_count() {
    use super::legend;
    use std::path::Path;
    let s = legend(Path::new("/tmp/project"), 3, 0, 40);
    assert!(s.contains("/tmp/project"), "{s}");
    assert!(s.contains("\u{00b7} 3"), "{s}");
}

#[test]
fn legend_shows_empty_for_a_zero_entry_dir() {
    use super::legend;
    use std::path::Path;
    let s = legend(Path::new("/tmp/empty"), 0, 0, 40);
    assert!(s.contains("\u{00b7} empty"), "{s}");
}

#[test]
fn legend_keeps_the_count_suffix_intact_when_the_path_is_truncated() {
    use super::legend;
    use std::path::Path;
    let long = Path::new("/very/long/nested/path/that/does/not/fit/at/all/here");
    let s = legend(long, 12, 0, 24);
    assert!(s.contains("\u{00b7} 12"), "count suffix dropped: {s}");
    assert!(s.contains('\u{2026}'), "path should be ellipsized: {s}");
}

#[test]
fn legend_shows_total_size_after_the_count() {
    use super::legend;
    use std::path::Path;
    let s = legend(Path::new("/tmp/project"), 3, 2048, 40);
    assert!(s.contains("\u{00b7} 3 \u{00b7} 2.0K"), "{s}");
}

#[test]
fn legend_says_empty_not_zero_bytes_for_a_zero_entry_dir() {
    use super::legend;
    use std::path::Path;
    let s = legend(Path::new("/tmp/empty"), 0, 0, 40);
    assert!(s.contains("\u{00b7} empty"), "{s}");
    assert!(
        !s.contains("0 B"),
        "must not fall back to the size suffix: {s}"
    );
}

#[test]
fn fmt_size_uses_compact_far_style_units() {
    use super::fmt_size;
    assert_eq!(fmt_size(0), "0 B");
    assert_eq!(fmt_size(427), "427 B");
    assert_eq!(fmt_size(1_229), "1.2K");
    assert_eq!(fmt_size(35_651_584), "34M");
    assert_eq!(fmt_size(2_254_857_830), "2.1G");
}

#[test]
fn file_rows_show_a_right_aligned_size() {
    let base = std::env::temp_dir().join("crew_far_render_size");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join("readme.md"), vec![b'x'; 1229]).unwrap();
    let pane = FarPane::new(base);
    let cells = render(&pane, 80, 24);
    let t = text(&cells);
    let row = t
        .lines()
        .find(|l| l.contains("readme.md"))
        .expect("file row rendered");
    assert!(row.contains("1.2K"), "size missing from row: {row:?}");
    // Right-aligned: the size's final glyph sits flush against a `│` border
    // cell. (Padding renders as absent blank cells, so text order alone
    // cannot show the gap.)
    let k = cells
        .iter()
        .filter(|c| c.c == 'K')
        .min_by_key(|c| (c.row, c.col))
        .expect("size unit cell rendered");
    assert!(
        cells
            .iter()
            .any(|c| c.row == k.row && c.col == k.col + 1 && c.c == '\u{2502}'),
        "size not flush at the panel's right border (K at col {})",
        k.col
    );
}

#[test]
fn overflowing_panel_paints_a_scroll_thumb_on_its_border() {
    let base = std::env::temp_dir().join("crew_far_render_thumb");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    for i in 0..30 {
        std::fs::write(base.join(format!("f{i:02}.txt")), b"x").unwrap();
    }
    let pane = FarPane::new(base);
    let cells = render(&pane, 40, 10);
    // BOTH panels show the same (overflowing) directory, so each must paint a
    // thumb on its own border. Asserting "any █" masked a bug where the right
    // panel's border render overwrote the left panel's thumb on the shared
    // column. The left panel's right border is the shared middle column
    // (col 20 for a 40-wide pane); the right panel's is the far-right (col 39).
    let thumb_cols: std::collections::BTreeSet<u16> = cells
        .iter()
        .filter(|c| c.c == '\u{2588}')
        .map(|c| c.col)
        .collect();
    assert!(
        thumb_cols.contains(&20),
        "left panel painted no thumb on the shared column (cols: {thumb_cols:?})"
    );
    assert!(
        thumb_cols.contains(&39),
        "right panel painted no thumb on its border (cols: {thumb_cols:?})"
    );
}

#[test]
fn short_listing_paints_no_scroll_thumb() {
    let cells = render(&fixture_pane("no_thumb"), 40, 24);
    assert!(
        cells.iter().all(|c| c.c != '\u{2588}'),
        "thumb painted though everything fits"
    );
}

#[test]
fn file_rows_show_a_type_glyph() {
    let base = std::env::temp_dir().join("crew_far_render_glyph");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("src")).unwrap();
    std::fs::write(base.join("main.rs"), b"x").unwrap();
    let pane = FarPane::new(base);
    let cells = render(&pane, 80, 24);
    // The rust glyph precedes a .rs file; the folder glyph precedes a dir.
    assert!(
        cells.iter().any(|c| c.c == '\u{e7a8}'),
        "no rust glyph rendered for main.rs"
    );
    assert!(
        cells.iter().any(|c| c.c == '\u{f07b}'),
        "no folder glyph rendered for the src/ dir"
    );
}

#[test]
fn ghost_text_renders_dim_after_the_cursor() {
    use crate::farpane::cmdhist::CmdHistory;
    let mut pane = fixture_pane("ghost");
    pane.cmdline = "ba".into();
    pane.history = CmdHistory::from_entries(vec!["bazqux".into()]);
    let cells = render(&pane, 80, 24);
    let cmd_row = 22; // rows(24) - cmdline row(1) - function bar row(1)
    let mut row: Vec<(u16, char, (u8, u8, u8))> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c, c.fg))
        .collect();
    row.sort_unstable_by_key(|(col, _, _)| *col);
    let line: String = row.iter().map(|(_, c, _)| *c).collect();
    assert!(line.contains("ba▏zqux"), "ghost text missing: {line:?}");
    let dim = crew_theme::theme().text_muted;
    let z_cell = row
        .iter()
        .find(|(_, c, _)| *c == 'z')
        .expect("ghost cell rendered");
    assert_eq!(z_cell.2, dim, "ghost text must render in text_muted");
}

#[test]
fn ghost_text_absent_when_history_does_not_extend_the_typed_text() {
    use crate::farpane::cmdhist::CmdHistory;
    let mut pane = fixture_pane("noghost");
    pane.cmdline = "yy".into();
    pane.history = CmdHistory::from_entries(vec!["bazqux".into()]);
    let cells = render(&pane, 80, 24);
    let cmd_row = 22;
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    assert!(
        !line.contains('z'),
        "no history entry should match 'yy': {line:?}"
    );
}

#[test]
fn ghost_hidden_while_a_tab_cycle_is_active() {
    use crate::farpane::cmdhist::CmdHistory;
    use crate::farpane::complete::CycleState;
    let mut pane = fixture_pane("ghostcycle");
    pane.cmdline = "ba".into();
    pane.history = CmdHistory::from_entries(vec!["bazqux".into()]);
    pane.complete = Some(CycleState {
        candidates: vec!["ba".into()],
        i: 0,
        prefix: "ba".into(),
    });
    let cells = render(&pane, 80, 24);
    let cmd_row = 22;
    let mut row: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == cmd_row)
        .map(|c| (c.col, c.c))
        .collect();
    row.sort_unstable_by_key(|(col, _)| *col);
    let line: String = row.into_iter().map(|(_, c)| c).collect();
    assert!(
        !line.contains('z'),
        "ghost must be suppressed during a completion cycle: {line:?}"
    );
}
