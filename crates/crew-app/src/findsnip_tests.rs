use super::*;

/// The characters of `label` at `hits`, as a string.
fn marked(label: &str, hits: &[usize]) -> String {
    let cs: Vec<char> = label.chars().collect();
    hits.iter().map(|&i| cs[i]).collect()
}

#[test]
fn a_short_row_is_left_whole_and_marks_its_match() {
    let (label, hits) = snippet("user", "what changed in plot/", "PLOT", 60);
    assert_eq!(label, "user: what changed in plot/");
    assert_eq!(marked(&label, &hits), "plot");
}

#[test]
fn a_match_past_the_edge_is_brought_into_view() {
    let text = format!("{} needle and more", "x".repeat(200));
    let room = 40;
    let (label, hits) = snippet("scout", &text, "needle", room);
    assert!(label.starts_with("scout: \u{2026}"), "{label}");
    assert_eq!(marked(&label, &hits), "needle");
    // The whole match is inside the columns the row has.
    assert!(hits.iter().all(|&i| i < room), "{hits:?} past {room}");
    // With some lead-in before it, not flush against the ellipsis.
    assert!(hits[0] > "scout: \u{2026}".chars().count());
}

#[test]
fn every_occurrence_in_view_is_marked() {
    let (label, hits) = snippet("a", "plot one, Plot two", "plot", 80);
    assert_eq!(marked(&label, &hits), "plotPlot");
}

#[test]
fn newlines_flatten_the_way_the_row_always_drew_them() {
    let (label, _) = snippet("a", "one\ntwo", "two", 80);
    assert_eq!(label, "a: one \u{23ce} two");
}

#[test]
fn no_match_is_the_plain_row() {
    let (label, hits) = snippet("a", "nothing here", "zzz", 80);
    assert_eq!(label, "a: nothing here");
    assert!(hits.is_empty());
}
