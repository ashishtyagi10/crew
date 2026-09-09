use super::*;
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
    // A mark, like a bullet: marker ink (was `text_muted`).
    assert_eq!(cell.fg, crate::chatink::marker_fg());
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

fn png(path: &std::path::Path, w: u32, h: u32) {
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x * 9) as u8, (y * 9) as u8, 120, 255])
    });
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(path, image::ImageFormat::Png)
        .expect("write");
}

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// Waits for the worker reading `path`; the frame never does this.
fn settle(path: &std::path::Path) {
    for _ in 0..400 {
        if crate::imgcache::probe(path) != crate::imgcache::Probe::Loading {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the worker never came back for {path:?}");
}

/// A remote picture, or a relative one with no directory to be relative to,
/// is named and nothing is read for it.
#[test]
fn a_remote_or_rootless_picture_is_only_named() {
    assert_eq!(fate("https://p.io/cat.png", None), Fate::Named);
    assert_eq!(fate("cat.png", None), Fate::Named);
    let dir = tmp_dir("chatimage-rootless");
    assert_eq!(fate("https://p.io/cat.png", Some(&dir)), Fate::Named);
}

/// On main `map_chat` did not exist and a card gave every picture ONE row;
/// here the one the cache holds gets the twelve rows of its box above the
/// caption — `out.len()` is 13, and rows 0..12 read back as the box.
#[test]
fn a_picture_the_cache_holds_gets_its_box_above_the_caption() {
    let _guard = crate::app::theme_test_guard();
    let dir = tmp_dir("chatimage-box");
    let img = dir.join("cat.png");
    png(&img, 16, 8);
    settle(&img);
    assert_eq!(fate("cat.png", Some(&dir)), Fate::Painted(img.clone()));
    let md = crate::md::render_chat("![a cat](cat.png)", 40);
    let out = crate::chatmd::map_chat(md, 40, (9, 9, 9), Some(&dir));
    assert_eq!(
        out.len(),
        13,
        "{:?}",
        out.iter().map(row_text).collect::<Vec<_>>()
    );
    for (i, row) in out[..12].iter().enumerate() {
        assert_eq!(box_row(row), Some((i as u16, "cat.png")), "row {i}");
        assert_eq!(row.len(), 41, "the box fills the card, to a click");
    }
    assert_eq!(row_text(&out[12]), " [image] a cat");
    assert_eq!(box_row(&out[12]), None, "the caption is not a box row");
}

/// The first ask for a path starts its read and answers `Loading` before
/// any worker can have finished: the card shows the caption and a muted
/// `loading…` under it. Once the read lands, the box.
#[test]
fn a_picture_still_being_read_says_so_under_its_caption() {
    let _guard = crate::app::theme_test_guard();
    let dir = tmp_dir("chatimage-loading");
    let img = dir.join("slow.png");
    png(&img, 8, 8);
    let md = crate::md::render_chat("![slow](slow.png)", 40);
    let out = crate::chatmd::map_chat(md.clone(), 40, (9, 9, 9), Some(&dir));
    let rows: Vec<String> = out.iter().map(row_text).collect();
    assert_eq!(rows, vec![" [image] slow", " loading\u{2026}"]);
    let muted = crew_theme::theme().text_muted;
    assert!(out[1][1..].iter().all(|c| c.fg == muted));
    settle(&img);
    let out = crate::chatmd::map_chat(md, 40, (9, 9, 9), Some(&dir));
    assert_eq!(out.len(), 13);
}

/// A file that is not a picture fails once and is the caption alone.
#[test]
fn a_picture_that_fails_to_decode_is_only_named() {
    let _guard = crate::app::theme_test_guard();
    let dir = tmp_dir("chatimage-bad");
    let bad = dir.join("notes.png");
    std::fs::write(&bad, "not a png").expect("write");
    settle(&bad);
    assert_eq!(fate("notes.png", Some(&dir)), Fate::Named);
    let md = crate::md::render_chat("![notes](notes.png)", 40);
    let out = crate::chatmd::map_chat(md, 40, (9, 9, 9), Some(&dir));
    assert_eq!(out.len(), 1);
    assert_eq!(row_text(&out[0]), " [image] notes");
}

/// A box row that grew a suffix — the compact clamp's ` … +N` — is no
/// longer a box row, so a folded card never paints twelve rows over the
/// cards under it.
#[test]
fn a_box_row_with_a_suffix_is_not_a_box_row() {
    let mut rows = box_lines("x.png", 10, (9, 9, 9));
    assert_eq!(box_row(&rows[3]), Some((3, "x.png")));
    rows[3].push(plain('+', (9, 9, 9), false));
    assert_eq!(box_row(&rows[3]), None);
}
