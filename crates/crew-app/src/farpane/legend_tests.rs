//! The panel legend keeps the folder's own name whole before any numbers.
use super::legend;

/// At every width, a legend that shows a count shows the folder's name
/// whole; the size goes first, then the count, and the name is cut only
/// when nothing else is left to give up.
#[test]
fn the_folder_name_outranks_the_count_and_size() {
    let path = "/Users/me/code/crew/crates/crew-app";
    for width in 8..=60 {
        let s = legend(path, 5, 7_800, width);
        if s.contains('\u{b7}') {
            assert!(
                s.contains("/crew-app") || s.contains(path),
                "{width}: {s:?}"
            );
        }
        assert!(
            s.chars().count() <= usize::from(width).saturating_sub(3),
            "{width}: {s:?}"
        );
    }
    assert_eq!(
        legend(path, 5, 7_800, 30),
        " \u{2026}/crew-app \u{b7} 5 \u{b7} 7.6K "
    );
    assert_eq!(legend(path, 5, 7_800, 22), " \u{2026}/crew-app \u{b7} 5 ");
    assert_eq!(legend(path, 5, 7_800, 16), " \u{2026}/crew-app ");
}

/// The thumb never takes the `│` beside a panel's corner: a card-scale
/// corner rounds by borrowing that cell, and with the thumb in it the
/// right panel's `╮` drew square (glass survey E4). At the top of the list
/// and at its bottom.
#[test]
fn the_thumb_leaves_the_corners_their_rule() {
    let base = std::env::temp_dir().join("crew_far_thumb_corner");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    for i in 0..30 {
        std::fs::write(base.join(format!("f{i:02}.txt")), b"x").unwrap();
    }
    let mut pane = crate::farpane::FarPane::new(base);
    for sel in [0, pane.right.entries.len() - 1] {
        pane.right.sel = sel;
        let cells = {
            let _lock = crate::app::THEME_LOCK.lock();
            crate::farpane::render::render(&pane, 40, 12)
        };
        let at = |row: u16| {
            cells
                .iter()
                .find(|c| c.col == 39 && c.row == row)
                .map(|c| c.c)
        };
        let top = cells
            .iter()
            .find(|c| c.col == 39 && c.c == '\u{256e}')
            .unwrap()
            .row;
        let bottom = cells
            .iter()
            .find(|c| c.col == 39 && c.c == '\u{256f}')
            .unwrap()
            .row;
        assert_eq!(at(top + 1), Some('\u{2502}'), "under the ╮ at sel {sel}");
        assert_eq!(at(bottom - 1), Some('\u{2502}'), "over the ╯ at sel {sel}");
        assert!(
            (top..bottom).any(|r| at(r) == Some('\u{2503}')),
            "no thumb at sel {sel}"
        );
    }
}
