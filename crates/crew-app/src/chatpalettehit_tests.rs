//! The chat pane's pickers mark what the query matched, like the bar's.
use super::*;
use crate::chatmention::MentionEntry;

fn marked(row: &MenuItem) -> String {
    row.hit
        .iter()
        .map(|&i| row.label.chars().nth(i).unwrap())
        .collect()
}

#[test]
fn a_filtered_slash_row_marks_its_prefix() {
    let rows = slash_items("do");
    let doctor = rows.iter().find(|r| r.label == "/doctor").expect("/doctor");
    assert_eq!(doctor.hit, [1, 2]);
    assert!(
        slash_items("").iter().all(|r| r.hit.is_empty()),
        "browsing marks nothing"
    );
}

#[test]
fn an_attach_row_marks_where_its_label_matched() {
    let entries = [
        MentionEntry::Skill {
            name: "verify".into(),
            desc: String::new(),
        },
        MentionEntry::File("crates/crew-app/src/render.rs".into()),
    ];
    let rows = attach_items("er", &entries, false);
    let skill = rows.iter().find(|r| r.label == "@skill:verify").unwrap();
    assert_eq!(marked(skill), "er", "in `verify`, not in `skill:`");
    assert_eq!(skill.hit, [8, 9]);
    let file = rows
        .iter()
        .find(|r| r.label.ends_with("render.rs"))
        .unwrap();
    assert_eq!(marked(file), "er");
}
