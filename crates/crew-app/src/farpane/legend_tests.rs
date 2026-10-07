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
