//! An opaque file has no text by design — it is not an empty file.
use crate::viewpane::detect::{Format, Opaque};
use crate::viewpane::{load::Loaded, LoadState, ViewPane};

fn text(cells: Vec<crew_render::CellView>) -> String {
    cells.iter().map(|c| c.c).collect()
}

#[test]
fn a_binary_file_shows_its_card_not_empty_file() {
    let _g = crate::app::theme_test_guard();
    let mut p = ViewPane::open(std::env::temp_dir().join("crew.dylib"));
    p.state = LoadState::Ready {
        format: Format::Opaque {
            why: Opaque::Binary,
        },
        loaded: Loaded {
            text: String::new(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    let shown = text(p.cells(60, 10));
    assert!(!shown.contains("(empty file)"), "{shown}");
    assert!(shown.contains("binary file"), "{shown}");
}
