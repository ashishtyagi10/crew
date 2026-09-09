use crate::chatbody::{CardLine, Color};

fn lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    crate::chatmd::map_lines(crate::md::render_chat(text, width), width, fg)
}

fn row_text(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

/// On main this was TWELVE blank rows (`md::picture::ROWS`), so `out.len()`
/// was 12 and no cell carried the source.
#[test]
fn an_image_paragraph_is_one_muted_row_naming_it_with_the_source_as_link() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("![a cat](https://p.io/cat.png)", 40, (9, 9, 9));
    assert_eq!(
        out.len(),
        1,
        "{:?}",
        out.iter().map(row_text).collect::<Vec<_>>()
    );
    assert_eq!(row_text(&out[0]), " [image] a cat");
    let cell = &out[0][2];
    assert_eq!(cell.fg, crew_theme::theme().text_muted);
    assert_eq!(cell.link.as_deref(), Some("https://p.io/cat.png"));
    assert!(out[0][1..].iter().all(|c| c.link.is_some()));
}

/// No alt text: the source names it, so the row is never a bare `[image]`.
#[test]
fn an_image_without_alt_is_named_by_its_source() {
    let _guard = crate::app::theme_test_guard();
    let out = lines("![](shot.png)", 40, (9, 9, 9));
    assert_eq!(row_text(&out[0]), " [image] shot.png");
}

/// The row is chunked to the card like any other line, and the prose
/// around a picture keeps its blank-row separation.
#[test]
fn the_image_row_wraps_and_sits_between_its_paragraphs() {
    let _guard = crate::app::theme_test_guard();
    let out = lines(
        "before\n\n![a long alt text here](x.png)\n\nafter",
        12,
        (9, 9, 9),
    );
    let rows: Vec<String> = out.iter().map(row_text).collect();
    assert_eq!(rows[0], " before");
    assert_eq!(rows[1], " ");
    assert_eq!(rows[2], " [image] a lo");
    assert_eq!(rows[3], " ng alt text ");
    assert_eq!(rows.last().map(String::as_str), Some(" after"));
    assert_eq!(rows.len(), 7, "{rows:?}");
}

/// The VIEWER still gets its picture box: `with_pictures` reserves the rows
/// and reports the placement, untouched by the chat collapse.
#[test]
fn the_viewer_path_still_reserves_the_picture_rows() {
    let _guard = crate::app::theme_test_guard();
    let md = crate::md::render("![a](x.png)", 40);
    let (rows, pics) = crate::chatmd::with_pictures(md, 40, (9, 9, 9));
    assert_eq!(rows.len(), 12);
    assert_eq!(
        pics,
        vec![crate::chatmd::Picture {
            row: 0,
            rows: 12,
            src: "x.png".into()
        }]
    );
}
