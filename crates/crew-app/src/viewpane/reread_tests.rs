//! A real file, a real worker thread: what the window does when the file
//! under it changed.
use crate::viewpane::caret::Step;
use crate::viewpane::ViewPane;

fn settle(p: &mut ViewPane) {
    for _ in 0..400 {
        if p.poll() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the load never landed");
}

#[test]
fn a_reread_drops_the_edits_and_the_undo_and_keeps_the_caret_on_its_byte() {
    let path = std::env::temp_dir().join(format!("crew-reread-{}.md", std::process::id()));
    std::fs::write(
        &path,
        "# One

first body
",
    )
    .unwrap();
    let mut p = ViewPane::open(path.clone());
    settle(&mut p);
    p.start_editing(60);
    for _ in 0..3 {
        p.move_caret(Step::Right, 60, 20);
    }
    p.insert("zz", 60, 20);
    assert!(p.dirty, "typed");
    assert!(p.source_str().unwrap().contains("Onezz"));
    let at = p.caret_at.expect("a caret");

    std::fs::write(
        &path,
        "# One

second body, longer than the first
",
    )
    .unwrap();
    p.reread();
    assert!(!p.dirty, "nothing unsaved any more");
    settle(&mut p);
    p.relayout_caret(60, 20);
    assert_eq!(
        p.source_str().unwrap(),
        "# One\n\nsecond body, longer than the first\n"
    );
    assert_eq!(p.caret_at, Some(at), "the byte survived");
    p.undo(60, 20);
    assert_eq!(
        p.source_str().unwrap(),
        "# One\n\nsecond body, longer than the first\n",
        "the old text's undo history is gone with the old text"
    );
    let _ = std::fs::remove_file(&path);
}

/// A caret past the end of a shorter file lands at the end, not nowhere.
#[test]
fn a_caret_past_the_new_end_lands_at_the_end() {
    let path = std::env::temp_dir().join(format!("crew-reread-short-{}.md", std::process::id()));
    std::fs::write(
        &path,
        "a much longer line of text
",
    )
    .unwrap();
    let mut p = ViewPane::open(path.clone());
    settle(&mut p);
    p.start_editing(60);
    p.move_caret(Step::End, 60, 20);
    assert!(p.caret_at.unwrap() > 10);
    std::fs::write(
        &path, "short
",
    )
    .unwrap();
    p.reread();
    settle(&mut p);
    p.relayout_caret(60, 20);
    assert!(p.caret.is_some(), "still an editor");
    assert!(
        p.caret_at.unwrap() <= 6,
        "at or before the new end: {:?}",
        p.caret_at
    );
    let _ = std::fs::remove_file(&path);
}
