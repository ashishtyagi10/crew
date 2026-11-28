use super::*;

const OLD: &str = "a\nb\nc\nd\ne\nf\ng\n";

#[test]
fn equal_text_has_no_hunks() {
    assert!(diff(OLD, OLD).is_empty());
}

#[test]
fn a_replacement_is_one_hunk_at_its_line() {
    let h = diff(OLD, "a\nb\nX\nd\ne\nf\ng\n");
    assert_eq!(
        h,
        vec![Hunk {
            at: 2,
            old: vec!["c\n".into()],
            new: vec!["X\n".into()]
        }]
    );
}

#[test]
fn an_insert_and_a_delete_are_hunks_of_their_own() {
    let h = diff(OLD, "a\nNEW\nb\nc\nd\ne\ng\n");
    assert_eq!(h.len(), 2);
    assert_eq!((h[0].at, h[0].old.len(), h[0].new.len()), (1, 0, 1));
    assert_eq!((h[1].at, h[1].old.len(), h[1].new.len()), (5, 1, 0));
}

#[test]
fn every_hunk_rebuilds_the_new_text_and_none_keeps_the_old() {
    let new = "x\nb\nc2\nd\nf\ng\nh\n";
    let h = diff(OLD, new);
    assert!(h.len() >= 3, "{h:?}");
    assert_eq!(apply(OLD, &h, |_| true), new);
    assert_eq!(apply(OLD, &h, |_| false), OLD);
}

#[test]
fn a_subset_is_exactly_the_original_plus_the_chosen_hunks() {
    let new = "x\nb\nc\nd\ne\nf\nG\n";
    let h = diff(OLD, new);
    assert_eq!(h.len(), 2);
    assert_eq!(apply(OLD, &h, |k| k == 1), "a\nb\nc\nd\ne\nf\nG\n");
    assert_eq!(apply(OLD, &h, |k| k == 0), "x\nb\nc\nd\ne\nf\ng\n");
}

#[test]
fn a_missing_final_newline_is_a_change() {
    let h = diff("a\nb\n", "a\nb");
    assert_eq!(h.len(), 1);
    assert_eq!(apply("a\nb\n", &h, |_| true), "a\nb");
}

#[test]
fn a_file_too_big_for_the_table_is_one_whole_hunk() {
    let old: String = (0..3000).map(|i| format!("o{i}\n")).collect();
    let new: String = (0..3000).map(|i| format!("n{i}\n")).collect();
    let h = diff(&old, &new);
    assert_eq!(h.len(), 1);
    assert_eq!(apply(&old, &h, |_| true), new);
}
