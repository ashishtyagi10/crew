use super::{dump_path, fmt_bytes, grid_row};
use crew_term::RenderCell;

#[test]
fn fmt_bytes_units() {
    assert_eq!(fmt_bytes(512), "512 B");
    assert_eq!(fmt_bytes(2048), "2 KB");
    assert_eq!(fmt_bytes(3_500_000), "3.3 MB");
}
use std::path::Path;

#[test]
fn dump_path_default_and_explicit() {
    let base = Path::new("/tmp/crewbase");
    // empty arg → timestamped name in the base dir.
    assert_eq!(
        dump_path("  ", base, "20260101-101010"),
        base.join("crew-dump-20260101-101010.txt")
    );
    // a relative arg joins the base; an absolute arg is kept as-is.
    assert_eq!(dump_path("log.txt", base, "s"), base.join("log.txt"));
    assert_eq!(
        dump_path("/var/out.txt", base, "s"),
        Path::new("/var/out.txt")
    );
}

fn cell(col: u16, row: u16, c: char) -> RenderCell {
    RenderCell {
        col,
        row,
        c,
        fg: (0, 0, 0),
        bg: (0, 0, 0),
        bold: false,
        italic: false,
        ..Default::default()
    }
}

#[test]
fn grid_row_reconstructs_and_trims() {
    // "hi" on row 1, with a gap and trailing spaces.
    let cells = [cell(0, 1, 'h'), cell(1, 1, 'i'), cell(5, 1, 'x')];
    assert_eq!(grid_row(&cells, 1, 10), "hi   x");
    // an empty row trims to nothing.
    assert_eq!(grid_row(&cells, 0, 10), "");
}

#[test]
fn grid_row_respects_column_bound() {
    // a cell past `cols` is ignored rather than panicking.
    let cells = [cell(0, 0, 'a'), cell(99, 0, 'z')];
    assert_eq!(grid_row(&cells, 0, 3), "a");
}
