//! A tag being typed, beside the completion offered for it.
use crate::todopane::tagmenu::TagMenu;

/// `@adm` wears the colour of the `@admin` the pop-up has highlighted — it
/// hashed to a colour of its own, so the tag about to be one read as two.
/// A tag already typed out, or one with no offer, keeps its own colour.
#[test]
fn a_partial_tag_wears_the_offered_completions_colour() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let mut p = crate::todopane::test_pane(Vec::new());
    p.input = "fix the build @adm".into();
    p.cursor = p.input.chars().count();
    p.tagmenu = Some(TagMenu {
        sigil: '@',
        matches: vec!["admin".into()],
        sel: 0,
    });
    let mut out = Vec::new();
    super::cells(&mut out, &p, 40, 12);
    let a = out.iter().find(|c| c.c == 'm').expect("the partial tag");
    assert_eq!(
        a.fg,
        crew_theme::tag_color("admin", t),
        "the offered colour"
    );
    p.tagmenu = None;
    out.clear();
    super::cells(&mut out, &p, 40, 12);
    let a = out.iter().find(|c| c.c == 'm').expect("the partial tag");
    assert_eq!(a.fg, crew_theme::tag_color("adm", t), "no offer: its own");
}
