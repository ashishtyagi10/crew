use crate::chatbody::{CardLine, Color};

fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    crate::chatmd::map_lines(crate::md::render_chat(text, width), width, fg)
}

fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// On main `~~gone~~` came out ITALIC in the body colour: `strike` did not
/// exist on the cell and `italic` was true.
#[test]
fn struck_text_wears_the_strike_and_steps_back_to_muted() {
    let _guard = crate::app::theme_test_guard();
    let fg = (9, 9, 9);
    let out = lines("keep ~~gone~~ keep", 40, fg);
    assert_eq!(row_text(&out[0]), " keep gone keep");
    let gone = &out[0][6];
    assert!(
        gone.strike && !gone.italic,
        "{:?}",
        (gone.strike, gone.italic)
    );
    assert_eq!(gone.fg, crew_theme::theme().text_muted);
    let keep = &out[0][1];
    assert!(!keep.strike);
    assert_eq!(keep.fg, fg);
}

/// A struck link keeps the link colour and its URL; only the rule is added.
#[test]
fn a_struck_link_keeps_its_colour_and_url() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("~~[old](https://o.io)~~", 40, (9, 9, 9));
    let cell = &out[0][1];
    assert!(cell.strike);
    assert_eq!(cell.fg, crate::chatink::link_color());
    assert_eq!(cell.link.as_deref(), Some("https://o.io"));
}

/// The strike reaches the renderer: `line_cells` puts it on the
/// `CellView`'s `deco`, which `crew-render` draws as a rule.
#[test]
fn the_strike_threads_through_to_the_cell_views_deco() {
    let _guard = crate::app::theme_test_guard();
    let page = crew_theme::theme().page_bg;
    let out = lines("~~x~~ y", 40, (9, 9, 9));
    let cells = crate::chatplace::line_cells(0, &out[0], 40, page);
    assert!(cells[1].deco.strike, "x");
    assert!(!cells[3].deco.strike, "y");
}

/// On main `heading_fg()` took no level and answered `ink` for all six.
#[test]
fn heading_ink_steps_down_by_level() {
    let _guard = crate::app::theme_test_guard();
    let th = crew_theme::theme();
    assert_eq!(super::heading_fg(1), crate::palette::accent());
    assert_eq!(super::heading_fg(2), th.ink);
    assert_eq!(super::heading_fg(3), th.text_muted);
    assert_eq!(super::heading_fg(6), th.text_muted);
    assert_ne!(super::heading_fg(1), super::heading_fg(2));
}

/// The h1 accent is the FLOORED accent: it clears the text floor against the
/// page on every preset, light ones included.
#[test]
fn the_h1_ink_is_readable_on_every_theme() {
    let _guard = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let ratio = crew_theme::contrast_ratio(super::heading_fg(1), crew_theme::theme().page_bg);
        assert!(
            ratio >= crew_theme::contrast::text_floor(),
            "{id:?}: {ratio:.2}"
        );
    }
}

/// On main `[^1]` was literal prose in the body colour.
#[test]
fn a_footnote_mark_is_muted_and_unlinked() {
    let _guard = crate::app::theme_test_guard();
    let fg = (9, 9, 9);
    let out = lines("a[^1]\n\n[^1]: n", 40, fg);
    assert_eq!(row_text(&out[0]), " a[1]");
    assert_eq!(out[0][2].fg, crew_theme::theme().text_muted);
    assert!(out[0][2].link.is_none() && !out[0][2].bold);
    assert_eq!(out[0][1].fg, fg);
    // The trailing block: a muted rule, then the marker-coloured `1. `.
    assert_eq!(out[2][1].c, '\u{2500}');
    assert_eq!(out[2][1].fg, crew_theme::theme().text_muted);
    assert_eq!(row_text(&out[3]), " 1. n");
    assert_eq!(out[3][1].fg, crate::chatink::marker_fg());
}
