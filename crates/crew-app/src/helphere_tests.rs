use super::*;
use crate::help::size;

/// A pane with keys of its own names its section; one whose keys are the
/// global chords (a terminal passes everything through) names none.
#[test]
fn a_pane_with_its_own_keys_names_its_section() {
    let far = PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir()));
    assert_eq!(section_for(&far), Some("in a /far file panel"));
    let todo = PaneContent::Todo(crate::todopane::test_pane(Vec::new()));
    assert_eq!(section_for(&todo), Some("in the /todo pane"));
}

/// Every section a pane can name is a heading the panel really draws — a
/// title that has drifted from `helplayout::sections` would scroll to the top
/// and say nothing about it.
#[test]
fn every_named_section_exists_in_the_panel() {
    let titles: Vec<&str> = crate::helplayout::sections()
        .iter()
        .map(|(t, _)| *t)
        .collect();
    let named = [
        "in an agent pane",
        "in the file viewer",
        "in a /far file panel",
        "in the /todo pane",
        "in /settings",
        "in the /disk map",
    ];
    for n in named {
        assert!(titles.contains(&n), "{n} is not a section any more");
        assert!(scroll_to(n, size().0) > 0, "{n} is not a row of the panel");
    }
}

/// The row it scrolls to IS that heading, not the one above or below it.
#[test]
fn the_scroll_lands_on_the_heading_itself() {
    let cols = size().0;
    let rows = crate::helplayout::rows("", cols);
    for (title, _) in crate::helplayout::sections() {
        let at = scroll_to(title, cols);
        assert!(
            matches!(&rows[at], crate::helplayout::Row::Head(h, _) if *h == title),
            "{title} landed on the wrong row"
        );
    }
}

/// A title nothing matches is the top, which is always a safe answer.
#[test]
fn an_unknown_section_is_the_top() {
    assert_eq!(scroll_to("in the hall of the mountain king", size().0), 0);
}
