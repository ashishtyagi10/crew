//! A viewer search shows its matches, not just how many lines hold them.
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

const TEXT: &str = "use crate::plot::sdf;\n\nfn arc() {\n    sdf::arc(p)\n}\n";

fn pane(needle: &str) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join("wash.rs"));
    p.state = LoadState::Ready {
        format: Format::Code { lang: "rust" },
        loaded: Loaded {
            text: TEXT.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    let lines: Vec<&str> = TEXT.lines().collect();
    let hits = crate::viewpane::search::find_matches(&lines, needle);
    p.search = Some(crate::viewpane::search::Search::new(needle.into(), hits));
    p
}

#[test]
fn every_match_on_screen_is_washed() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let (cols, rows) = (60u16, 12u16);
    let cells = pane("sdf").cells(cols, rows);
    let washed: Vec<_> = cells
        .iter()
        .filter(|c| c.bg == t.find_hl_bg && c.row + 1 < rows)
        .collect();
    assert_eq!(washed.len(), 6, "both `sdf`s, three cells each");
    assert!(washed.iter().all(|c| "sdf".contains(c.c)));
    // …and the line saying so is ink, not a tick's 3:1 olive.
    let status: Vec<_> = cells
        .iter()
        .filter(|c| c.row == rows - 1 && c.c != ' ')
        .collect();
    assert!(
        status.iter().all(|c| c.fg == t.ink),
        "the search line reads as text"
    );
}
