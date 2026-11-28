use super::super::hunkdiff::diff;
use super::*;

fn shown(old: &str, new: &str) -> String {
    let (lead, body) = show(Some(old), new, &diff(old, new));
    format!("{lead}{body}")
}

#[test]
fn a_replacement_and_an_insert_are_numbered_hunks() {
    let old = "a\nlet x = 1;\nb\nc\nd\ne\n";
    let new = "a\nlet x = 2;\nb\nc\nd\ne\nfn new() {}\n";
    assert_eq!(
        shown(old, new),
        "2 changes \u{b7} \n[1] @@ 2 @@\n- let x = 1;\n+ let x = 2;\n[2] @@ 7 @@\n+ fn new() {}\n\
         Enter all \u{b7} Esc none \u{b7} type 1 3 to keep those"
    );
}

#[test]
fn a_delete_shows_only_minus_rows() {
    let s = shown("a\nb\nc\n", "a\nc\n");
    assert!(s.starts_with("1 change \u{b7} \n[1] @@ 2 @@\n- b\n"), "{s}");
    assert!(s.ends_with("Enter allows \u{b7} Esc refuses"), "{s}");
}

#[test]
fn a_new_file_says_so_and_shows_its_first_lines() {
    let new: String = (1..=60).map(|i| format!("l{i}\n")).collect();
    let (lead, body) = show(None, &new, &[]);
    assert_eq!(lead, "new file, 60 lines \u{b7} ");
    let rows: Vec<&str> = body.lines().skip(1).collect();
    assert_eq!(rows[0], "+ l1");
    assert_eq!(rows[39], "+ l40");
    assert_eq!(rows[40], "\u{2026} 20 more lines");
    assert_eq!(rows.len(), 42);
}

#[test]
fn a_long_diff_is_clipped_at_forty_rows_with_a_marker() {
    let old: String = (0..30).map(|i| format!("o{i}\n")).collect();
    let new: String = (0..30).map(|i| format!("n{i}\n")).collect();
    let s = shown(&old, &new);
    assert!(s.contains("\u{2026} 21 more lines"), "{s}");
    assert_eq!(s.lines().filter(|l| l.starts_with(['-', '+'])).count(), 39);
    // Hunks past the clip were never shown, so none can be picked: all or nothing.
    assert!(s.ends_with("Enter allows \u{b7} Esc refuses"), "{s}");
}

#[test]
fn many_hunks_clipped_short_offer_no_picks() {
    let old: String = (0..25).map(|i| format!("o{i}\nkeep\n")).collect();
    let new: String = (0..25).map(|i| format!("n{i}\nkeep\n")).collect();
    let s = shown(&old, &new);
    assert!(s.starts_with("25 changes"), "{s}");
    assert!(s.contains("more lines"), "{s}");
    assert!(s.ends_with("Enter allows \u{b7} Esc refuses"), "{s}");
}

#[test]
fn several_hunks_all_on_screen_offer_picks() {
    let s = shown(
        "a
x
b
", "A
x
B
",
    );
    assert!(s.ends_with(crate::broker::hunkpicks::PICK_KEYS), "{s}");
}

#[test]
fn a_long_line_is_cut_at_120_characters() {
    let s = shown("a\n", &format!("{}\n", "x".repeat(300)));
    let plus = s.lines().find(|l| l.starts_with("+ ")).unwrap();
    assert_eq!(plus.chars().count(), 2 + 120 + 1);
    assert!(plus.ends_with('\u{2026}'));
}
