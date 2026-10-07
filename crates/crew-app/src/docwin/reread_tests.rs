use super::guard;

#[test]
fn a_clean_document_rereads_at_once_and_a_dirty_one_asks_exactly_once() {
    assert_eq!(guard(false, false), Ok(()));
    assert_eq!(guard(false, true), Ok(()));
    let ask = guard(true, false).unwrap_err();
    // The window's own chords, as this platform writes them.
    let (r, s) = (
        crate::chordglyph::shown_ctrl("Cmd+R"),
        crate::chordglyph::shown_ctrl("Cmd+S"),
    );
    assert!(
        ask.contains(&format!("{r} again")) && ask.contains(s),
        "{ask}"
    );
    assert_eq!(guard(true, true), Ok(()), "the second press goes ahead");
}
