use crate::viewpane::caret::Step;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

/// What is selected in the viewer wears the terminal's selection, never the
/// find highlight: on the Ink Reader's dark page that was a yellow wash, a
/// selection that read as a search hit (UI survey J9).
#[test]
fn a_selection_in_the_viewer_is_washed_as_a_selection() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    let mut p = ViewPane::open(std::env::temp_dir().join("selwash.md"));
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: "pick these words\n".into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p.start_editing(60);
    p.anchor_here();
    for _ in 0..4 {
        p.move_caret(Step::Right, 60, 20);
    }
    let t = crew_theme::theme();
    let want = crew_theme::readable::selection_bg(t);
    assert_ne!(want, t.find_hl_bg, "the page tells them apart");
    let cells = p.cells(60, 20);
    let washed = cells.iter().filter(|c| c.bg == want).count();
    assert_eq!(washed, 4, "the four selected letters");
    assert!(cells.iter().all(|c| c.bg != t.find_hl_bg));
}
