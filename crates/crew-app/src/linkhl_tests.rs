use super::{colorize, link_fg, DecoLine};
use crew_render::CellView;

fn row(text: &str) -> Vec<CellView> {
    text.chars()
        .enumerate()
        .map(|(i, c)| CellView {
            col: i as u16,
            row: 0,
            c,
            fg: (200, 200, 200),
            bg: (0, 0, 0),
            bold: false,
            italic: false,
            ..Default::default()
        })
        .collect()
}

#[test]
fn tints_only_url_cells() {
    // The link colour is derived from the live theme; guard the global.
    let _g = crate::app::theme_test_guard();
    let line = "go https://ex.io/x done";
    let mut cells = row(line);
    let n = colorize(&mut cells, line.len() as u16, 1);
    let url = "https://ex.io/x";
    assert_eq!(n, url.len());
    // ...and ruled, so the link is legible without reading the hue.
    let ruled = cells.iter().filter(|c| c.deco.line == DecoLine::Single);
    assert_eq!(ruled.count(), url.len());
    // every URL cell is tinted...
    let start = line.find(url).unwrap();
    for c in &cells {
        let in_url = (start..start + url.len()).contains(&(c.col as usize));
        assert_eq!(c.fg == link_fg(), in_url, "col {} mismatch", c.col);
    }
}

#[test]
fn no_url_leaves_colors_untouched() {
    // The link colour is derived from the live theme; guard the global.
    let _g = crate::app::theme_test_guard();
    let mut cells = row("just plain text");
    assert_eq!(colorize(&mut cells, 15, 1), 0);
    assert!(cells.iter().all(|c| c.fg != link_fg()));
}
