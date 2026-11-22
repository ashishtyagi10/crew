//! The shared editing keys, applied to a pane.
use crate::docwin::keys::Edit;
use crate::viewpane::caret::Step;
use crate::viewpane::editpane_tests::doc;
use crate::viewpane::EditDone;

/// `i` starts where the reader is, not back at the top of the file.
#[test]
fn editing_starts_at_the_top_of_what_is_on_screen() {
    let mut p = doc();
    p.scroll = 4;
    p.edit_here(60, 6);
    let c = p.caret.expect("editing");
    assert!(
        c.row >= 4,
        "caret on row {} with the view scrolled to 4",
        c.row
    );
}

#[test]
fn a_raw_view_is_not_edited() {
    let mut p = doc();
    p.raw = true;
    p.edit_here(60, 6);
    assert!(
        p.caret.is_none(),
        "`s` shows bytes the caret has no provenance for"
    );
}

#[test]
fn stopping_keeps_the_edits_and_drops_the_caret() {
    let mut p = doc();
    p.edit_here(60, 20);
    assert_eq!(
        p.apply_edit(Edit::Type("Z".into()), 60, 20),
        EditDone::Drawn
    );
    p.stop_editing();
    assert!(p.caret.is_none() && p.dirty);
    assert!(p.source_str().unwrap().contains('Z'));
    assert_eq!(
        p.title(),
        "caret.md \u{25cf}",
        "the pane's title says it is unsaved"
    );
}

/// A refused close is re-armed by typing, and only by typing.
#[test]
fn typing_re_arms_the_unsaved_warning_and_moving_does_not() {
    let mut p = doc();
    p.edit_here(60, 20);
    p.warned = true;
    p.apply_edit(Edit::Move(Step::Right), 60, 20);
    assert!(p.warned);
    p.apply_edit(Edit::Type("a".into()), 60, 20);
    assert!(!p.warned);
}

#[test]
fn the_host_s_share_of_an_edit_comes_back_to_it() {
    let mut p = doc();
    p.edit_here(60, 20);
    assert_eq!(p.apply_edit(Edit::Save, 60, 20), EditDone::Save);
    assert_eq!(p.apply_edit(Edit::Paste, 60, 20), EditDone::Paste);
    p.apply_edit(Edit::Select(Step::WordRight), 60, 20);
    assert!(matches!(p.apply_edit(Edit::Copy, 60, 20), EditDone::Copy(t) if !t.is_empty()));
}
