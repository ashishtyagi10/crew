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
