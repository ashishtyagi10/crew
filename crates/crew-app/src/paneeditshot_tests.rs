//! Off-screen shot of a markdown viewer pane being EDITED in the grid (`i`): the caret's block
//! drawn as its markdown, the rest rendered, beside another pane.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew pane_edit_shot -- --ignored`

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pane_edit_shot() {
    let _g = crate::app::theme_test_guard();
    let dir = std::env::temp_dir().join("pane-edit-shot");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("notes.md");
    let text = "# Notes\n\nSome **bold** words and a [link](https://example.invalid) here.\n\n\
                - first item\n- second item\n\n> a quote\n";
    std::fs::write(&f, text).unwrap();
    let view = format!("/view {}", f.display());
    let at = text.find("words").unwrap() as u32;
    let prep = move |app: &mut crate::app::CrewApp| {
        let pane = &mut app.panes[1];
        let (cols, rows) = (pane.grid.cols, pane.grid.rows);
        if let crate::pane::PaneContent::View(v) = &mut pane.content {
            v.edit_here(cols, rows);
            v.caret_at = Some(at);
            v.relayout_caret(cols, rows);
        }
    };
    for id in [crew_theme::ThemeId::PaperLight, crew_theme::current_id()] {
        let name = format!("pane-edit-{id:?}").to_lowercase();
        crate::crtglassshot_tests::window_with(&name, id, 1.0, &["/far", &view], &prep);
    }
}
