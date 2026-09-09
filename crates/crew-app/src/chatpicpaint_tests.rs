use super::*;

fn png(path: &std::path::Path, w: u32, h: u32) {
    let img = image::RgbaImage::from_fn(w, h, |_, _| image::Rgba([200, 40, 40, 255]));
    image::DynamicImage::ImageRgba8(img)
        .save_with_format(path, image::ImageFormat::Png)
        .expect("write");
}

fn tmp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

/// The box rows `from..to` of a picture, placed from row `at` down.
fn placed_box(src: &str, from: usize, to: usize, at: u16) -> Vec<(u16, CardLine)> {
    crate::chatimage::box_lines(src, 8, (9, 9, 9))
        .into_iter()
        .skip(from)
        .take(to - from)
        .enumerate()
        .map(|(k, l)| (at + k as u16, l))
        .collect()
}

/// A box cut by the top of the window is anchored where its first row
/// WOULD be — above the window — not on its first visible row.
#[test]
fn a_box_is_anchored_at_its_first_row_even_when_that_row_is_off_screen() {
    let mut placed = placed_box("x.png", 3, 12, 5);
    placed.push((14, vec![crate::chatbody::plain('c', (9, 9, 9), false)]));
    assert_eq!(boxes(&placed), vec![(2, "x.png".to_string())]);
}

/// Two pictures back to back are two boxes; the same picture twice is too.
#[test]
fn consecutive_boxes_are_told_apart() {
    let mut placed = placed_box("a.png", 0, 12, 0);
    placed.extend(placed_box("a.png", 0, 12, 12));
    assert_eq!(
        boxes(&placed),
        vec![(0, "a.png".to_string()), (12, "a.png".to_string())]
    );
}

/// The paint pass reads only what has landed: a path never asked for is
/// not asked for here either, so no worker starts from a paint.
#[test]
fn the_paint_pass_never_starts_a_read() {
    let dir = tmp_dir("chatpicpaint-cold");
    let img = dir.join("cold.png");
    png(&img, 4, 4);
    let placed = placed_box("cold.png", 0, 12, 0);
    let out = paint(&placed, 40, 2.0, Some(&dir), (0.0, 0.0, 40.0, 12.0));
    assert!(out.is_empty());
    assert!(
        !crate::imgcache::pending(&img),
        "a paint must not ask the disk"
    );
}

/// A landed picture is painted inside its box, one column in from each
/// edge, and clipped to the window it was given.
#[test]
fn a_landed_picture_is_painted_into_its_box_and_clipped() {
    let dir = tmp_dir("chatpicpaint-warm");
    let img = dir.join("warm.png");
    png(&img, 40, 20);
    for _ in 0..400 {
        if crate::imgcache::probe(&img) != crate::imgcache::Probe::Loading {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(crate::imgcache::probe(&img), crate::imgcache::Probe::Ready);
    let placed = placed_box("warm.png", 0, 12, 4);
    let full = paint(&placed, 40, 2.0, Some(&dir), (0.0, 4.0, 40.0, 16.0));
    assert!(!full.is_empty(), "a landed picture is drawn");
    for p in &full {
        assert!(p.x >= 1.0 && p.x + p.w <= 39.0, "inside the card: {p:?}");
        assert!(
            p.y >= 4.0 && p.y + p.h <= 16.0 + 1e-3,
            "inside the box: {p:?}"
        );
    }
    // Half the window: everything above row 10 is clipped away.
    let cut = paint(&placed, 40, 2.0, Some(&dir), (0.0, 10.0, 40.0, 16.0));
    assert!(!cut.is_empty() && cut.len() <= full.len());
    assert!(cut.iter().all(|p| p.y >= 10.0 - 1e-3), "{cut:?}");
}
