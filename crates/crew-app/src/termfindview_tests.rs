use super::paint;
use crate::termfind::TermFind;
use crew_render::CellView;
use crew_term::{GridSize, HeadlessTerm, TermModel};
use crew_theme::ThemeId;

const W: u16 = 30;
const H: u16 = 6;

/// A `W`x`H` pane drawn from `lines`, on the terminal's own colours, with a
/// bright-on-red SGR background across row 3 (a TUI's painted row).
fn pane(lines: &[&str]) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut cells = Vec::new();
    for (r, line) in lines.iter().enumerate() {
        for (col, c) in line.chars().enumerate() {
            let painted = r == 3;
            cells.push(CellView {
                col: col as u16,
                row: r as u16,
                c,
                fg: if painted { (255, 255, 255) } else { t.term_fg },
                bg: if painted { (200, 40, 40) } else { t.term_bg },
                ..Default::default()
            });
        }
    }
    cells
}

fn find(query: &str, text: &str, sel: usize) -> TermFind {
    let mut term = HeadlessTerm::new(GridSize { cols: W, rows: H });
    term.feed(text.as_bytes());
    let mut f = TermFind::default();
    query.chars().for_each(|c| f.edit(Some(c)));
    f.set_matches(term.find_all(query), term.screen_line());
    f.sel = sel;
    f
}

const LINES: [&str; 5] = [
    "",
    "one needle two",
    "needle again",
    "error: needle on red",
    "last",
];
const TEXT: &str = "\r\none needle two\r\nneedle again\r\nerror: needle on red\r\nlast";

fn at(cells: &[CellView], row: u16, col: u16) -> &CellView {
    cells.iter().find(|c| c.row == row && c.col == col).unwrap()
}

fn drawn(id: ThemeId, sel: usize, query: &str) -> Vec<CellView> {
    crew_theme::set_theme(id);
    let mut cells = pane(&LINES);
    paint(&mut cells, &find(query, TEXT, sel), 0, W, H);
    cells
}

#[test]
fn the_current_match_is_stronger_and_every_match_stays_readable() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::PaperDark, ThemeId::PaperLight] {
        // Matches newest first: sel 0 is the red row's, sel 2 the oldest.
        let cells = drawn(id, 0, "needle");
        let page = crew_theme::theme().term_bg;
        let (cur, other) = (at(&cells, 3, 8), at(&cells, 1, 4));
        assert_ne!(cur.bg, other.bg, "{id:?}: current differs from the rest");
        assert_ne!(other.bg, page, "{id:?}: the rest are tinted");
        assert!(cur.mark && other.mark);
        let floor = crew_theme::contrast::text_floor();
        for c in cells.iter().filter(|c| c.row > 0 && c.bg != page) {
            let r = crew_theme::contrast_ratio(c.fg, c.bg);
            assert!(
                r >= floor - 0.05,
                "{id:?} ({},{}): {r:.2} < {floor}",
                c.row,
                c.col
            );
        }
        // Outside a match nothing moved, painted rows included.
        assert_eq!(at(&cells, 3, 0).bg, (200, 40, 40), "{id:?}");
        assert_eq!(at(&cells, 1, 0).bg, page, "{id:?}");
        // Stepping moves the strong tint to the other match.
        let stepped = drawn(id, 2, "needle");
        assert_eq!(at(&stepped, 1, 4).bg, cur.bg, "{id:?}: now the current");
        assert_ne!(
            at(&stepped, 3, 8).bg,
            cur.bg,
            "{id:?}: the red row's match eased"
        );
    }
}

#[test]
fn the_bar_shows_query_and_count_right_aligned_and_no_matches_in_bell() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in [ThemeId::PaperDark, ThemeId::PaperLight] {
        let cells = drawn(id, 1, "needle");
        let row = |cs: &[CellView]| -> String {
            let mut r: Vec<_> = cs.iter().filter(|c| c.row == 0).collect();
            r.sort_by_key(|c| c.col);
            r.iter().map(|c| c.c).collect()
        };
        let text = row(&cells);
        assert_eq!(text.chars().count(), W as usize, "the bar spans the pane");
        assert!(text.starts_with(" find needle"), "{id:?}: {text:?}");
        assert!(text.ends_with("2/3 "), "{id:?}: {text:?}");
        let none = drawn(id, 0, "zzz");
        assert!(row(&none).ends_with("no matches "));
        let c = at(&none, 0, W - 3);
        let bell = crew_theme::theme().bell;
        assert!(crew_theme::contrast_ratio(c.fg, c.bg) >= 3.0 - 0.05);
        assert_eq!(c.fg, crew_theme::readable::enforced(bell, c.bg, 3.0));
    }
}

#[test]
fn a_match_on_the_top_row_moves_the_bar_to_the_bottom() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(ThemeId::PaperDark);
    // Scrolled so the current match is on row 0.
    let long = format!("{TEXT}{}", "\r\nx".repeat(10));
    let f = find("needle", &long, 0);
    let m = f.matches[0];
    let off = (-i64::from(m.line)).max(0) as usize;
    let mut cells = pane(&LINES);
    paint(&mut cells, &f, off, W, H);
    assert_eq!(f.bar_row(off, H), H - 1);
    assert!(
        cells.iter().any(|c| c.row == H - 1 && c.c == 'f'),
        "bar below"
    );
}
