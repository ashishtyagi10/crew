use super::{bytes, label_ink, tile_bg, tiles, Child, DiskPane, MIN_PATH_W};

fn kids(sizes: &[(&str, u64, bool)]) -> Vec<Child> {
    sizes
        .iter()
        .map(|(n, b, d)| Child {
            name: (*n).into(),
            bytes: *b,
            is_dir: *d,
        })
        .collect()
}

#[test]
fn a_tiles_share_of_the_pane_is_its_share_of_the_bytes() {
    let c = kids(&[("big", 800, true), ("small", 200, true)]);
    let t = tiles(&c, 40, 20);
    let area = |i: usize| {
        let t = t.iter().find(|t| t.index == i).unwrap();
        t.w * t.h
    };
    let ratio = area(0) / (area(0) + area(1));
    assert!((ratio - 0.8).abs() < 0.02, "80% of the bytes: {ratio}");
}

#[test]
fn bytes_are_four_characters_wherever_they_can_be() {
    assert_eq!(bytes(0), "0B");
    assert_eq!(bytes(900), "900B");
    assert_eq!(bytes(9_216), "9k");
    assert_eq!(bytes(5 * 1024 * 1024), "5M");
    assert_eq!(bytes(4_509_715_660), "4.2G");
    assert!(bytes(u64::MAX).len() <= 8, "{}", bytes(u64::MAX));
}

#[test]
fn the_selection_wraps_in_size_order() {
    let mut p = DiskPane::new(std::env::temp_dir());
    p.children = kids(&[("a", 3, true), ("b", 2, true), ("c", 1, false)]);
    assert_eq!(p.selected, 0);
    p.selected = 2;
    // Past the end wraps to the biggest again; the order is the list's,
    // which is size order.
    p.selected = (p.selected + 1) % p.children.len();
    assert_eq!(p.selected, 0);
    p.selected = p.selected.checked_sub(1).unwrap_or(p.children.len() - 1);
    assert_eq!(p.selected, 2);
}

#[test]
fn only_a_directory_can_be_descended_into() {
    let mut p = DiskPane::new(std::env::temp_dir());
    p.children = kids(&[("dir", 3, true), ("file.txt", 2, false)]);
    assert!(p.child(0).filter(|c| c.is_dir).is_some());
    assert!(p.child(1).filter(|c| c.is_dir).is_none());
}

#[test]
fn walking_up_from_the_root_stays_at_the_root() {
    let mut p = DiskPane::new(std::path::PathBuf::from("/"));
    p.open(None);
    assert_eq!(p.root(), std::path::Path::new("/"));
}

#[test]
fn a_real_directory_is_walked_and_totalled() {
    // The one test that actually touches the filesystem: a scan has to
    // produce the sizes it claims, and the pane has to notice.
    let dir = std::env::temp_dir().join(format!("crew-disk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("a.bin"), vec![0u8; 4096]).unwrap();
    std::fs::write(dir.join("sub").join("b.bin"), vec![0u8; 1024]).unwrap();

    let mut p = DiskPane::new(dir.clone());
    // The walk is on a worker thread; poll until it reports done.
    let start = std::time::Instant::now();
    while p.is_scanning() && start.elapsed() < std::time::Duration::from_secs(5) {
        p.poll();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    p.poll();
    assert!(!p.is_scanning(), "the scan finished");
    let mut names: Vec<(&str, u64)> = p
        .children()
        .iter()
        .map(|c| (c.name.as_str(), c.bytes))
        .collect();
    names.sort();
    assert_eq!(names, vec![("a.bin", 4096), ("sub", 1024)]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_click_lands_on_the_tile_it_is_over() {
    let mut p = DiskPane::new(std::env::temp_dir());
    p.set_children_for_test(&[("big", 800, true), ("small", 200, true)], 0);
    let t = tiles(p.children(), 40, 20);
    for tile in &t {
        let (cx, cy) = (tile.x + tile.w / 2.0, tile.y + tile.h / 2.0);
        assert_eq!(p.tile_at(cx, cy, 40, 20), Some(tile.index));
    }
    // The header rows are not the map.
    assert_eq!(p.tile_at(1.0, 0.0, 40, 20), None);
}

/// A treemap tile that cuts a name without saying so reads as a complete,
/// wrong name: `vendor` in a small tile drew `vend`, which is not a
/// directory anybody has.
#[test]
fn a_tile_too_narrow_for_a_name_says_the_name_is_cut() {
    let _g = crate::app::theme_test_guard();
    let mut p = DiskPane::new(std::env::temp_dir());
    // One huge tile and one small one, so the small tile is narrow.
    p.seed_children(kids(&[("target", 760, true), ("vendor", 240, true)]));
    let text = |cells: &[crew_render::CellView]| -> String {
        let mut v: Vec<&crew_render::CellView> = cells.iter().collect();
        v.sort_by_key(|c| (c.row, c.col));
        v.iter().map(|c| c.c).collect()
    };
    let drawn = text(&p.cells(26, 14));
    assert!(
        drawn.contains("target"),
        "the big tile keeps its whole name: {drawn:?}"
    );
    // The narrow tile marks its cut rather than drawing `vend`, which
    // would read as a complete name for a directory nobody has.
    assert!(
        drawn.contains("ven\u{2026}"),
        "a cut name drew as a complete wrong one: {drawn:?}"
    );
    assert!(
        !drawn.contains("vendor"),
        "it really did not fit: {drawn:?}"
    );
}

/// Every tile's label must read against the tile it is written on. The
/// picked tile used to take the page's own ink — near-white on a dark
/// theme — and write it on a bright pastel fill, which made the one tile
/// you had selected the one label on the map you could not read.
#[test]
fn every_tile_label_reads_against_its_own_tile() {
    let _g = crate::app::theme_test_guard();
    for id in [
        crew_theme::ThemeId::PaperDark,
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::CrtGreen,
    ] {
        crew_theme::set_theme(id);
        let children = kids(&[
            ("target", 4_509_715_660, true),
            ("crates", 812_000_000, true),
            (".git", 402_000_000, true),
            ("vendor", 121_000_000, true),
            ("Cargo.lock", 310_000, false),
        ]);
        let map = tiles(&children, 120, 40);
        let colors = super::tile_colors(&children, &map);
        for (i, child) in children.iter().enumerate() {
            for selected in [false, true] {
                let bg = tile_bg(colors[i], child.is_dir, selected);
                let ratio = crew_theme::contrast_ratio(label_ink(bg), bg);
                assert!(
                    ratio >= crew_theme::contrast::text_floor(),
                    "{id:?} tile {i} ({}) selected={selected}: {ratio:.2}",
                    child.name
                );
            }
        }
    }
}

/// No two tiles that touch may share a colour. The pool is six entries
/// picked by name hash; a directory with eight children collides by the
/// pigeonhole principle, and two neighbouring tiles the same colour read
/// as one region — the one thing a map of areas is for.
#[test]
fn touching_tiles_never_share_a_colour() {
    let _g = crate::app::theme_test_guard();
    for id in [
        crew_theme::ThemeId::PaperDark,
        crew_theme::ThemeId::PaperLight,
        crew_theme::ThemeId::CrtGreen,
    ] {
        crew_theme::set_theme(id);
        // The repo's own root, which is where the collision was found.
        let children = kids(&[
            ("target", 4_509_715_660, true),
            ("crates", 812_000_000, true),
            (".git", 402_000_000, true),
            ("vendor", 121_000_000, true),
            ("docs", 24_000_000, true),
            ("Cargo.lock", 310_000, false),
            ("CHANGELOG.md", 96_000, false),
            ("README.md", 12_000, false),
        ]);
        for (cols, rows) in [(40u16, 20u16), (80, 30), (160, 50)] {
            let map = tiles(&children, cols, rows);
            let colors = super::tile_colors(&children, &map);
            for (i, a) in map.iter().enumerate() {
                for (j, b) in map.iter().enumerate() {
                    if i < j && super::touches(a, b) {
                        assert_ne!(
                            colors[i], colors[j],
                            "{id:?} {cols}x{rows}: {} and {} touch and match",
                            children[a.index].name, children[b.index].name
                        );
                    }
                }
            }
        }
    }
}

/// The header's two numbers are what it exists to say. They used to be
/// glued to the front of a path and clipped off the right edge of a narrow
/// pane together with half the path.
#[test]
fn the_header_keeps_its_reading_however_narrow_the_pane() {
    let _g = crate::app::theme_test_guard();
    let mut p = DiskPane::new(std::path::PathBuf::from(
        "/var/folders/wm/z5pvj2457dqfv9c1kzfcb1y00000gn/T/scratch",
    ));
    p.set_children_for_test(
        &[("target", 4_000_000_000, true), ("docs", 24_000, true)],
        0,
    );
    for cols in [24u16, 40, 60, 120] {
        let row: String = {
            let mut v: Vec<_> = p
                .cells(cols, 30)
                .into_iter()
                .filter(|c| c.row == 1)
                .collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert!(
            row.contains("2 entries"),
            "{cols} cols lost the reading: {row:?}"
        );
        assert!(
            crate::chatwidth::str_w(&row) < usize::from(cols),
            "{cols} cols overflowed: {row:?}"
        );
        // Wide enough for a path, and it is the TAIL that survives: the
        // directory you are in, not the road you took to it.
        if cols >= 40 {
            assert!(
                row.contains("scratch"),
                "{cols} cols lost the tail: {row:?}"
            );
        }
    }
    // …and the floor is real: below it the reading stands alone rather
    // than sharing with an ellipsis and two characters.
    const { assert!(MIN_PATH_W >= 4) };
}
