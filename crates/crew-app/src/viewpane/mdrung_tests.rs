use super::*;

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

#[test]
fn markdown_renders_rather_than_showing_its_markers() {
    // The whole point of deleting the source half: a heading reads as a
    // heading, not as "# heading".
    let ls = lines("# Title\n\nbody text\n", 40);
    let all: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(all.contains("Title"));
    assert!(!all.contains("# Title"), "markers are rendered away: {all}");
}

#[test]
fn a_heading_is_bold_and_the_body_is_not() {
    let ls = lines("# Title\n\nbody\n", 40);
    let head = ls.iter().find(|l| text(l).contains("Title")).unwrap();
    let body = ls.iter().find(|l| text(l).contains("body")).unwrap();
    assert!(head.iter().any(|c| c.bold), "heading is emphasised");
    assert!(!body.iter().any(|c| c.bold), "body is not");
}

#[test]
fn content_wraps_inside_the_pane_width() {
    // A single unbroken run, so map_lines must hard-wrap at exactly the
    // content width — the only case where the one-column indent
    // compensation is observable. Word-wrapped prose leaves slack at the
    // line end and passes whether or not the compensation is there.
    let ls = lines(&format!("{}\n", "x".repeat(200)), 30);
    for l in &ls {
        let w: usize = l.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
        assert!(w <= 30, "line of width {w} exceeds 30: {:?}", text(l));
    }
}

#[test]
fn a_link_keeps_its_url_on_the_cells() {
    // clickopen recovers the URL from the cell rather than re-parsing.
    let ls = lines("[crew](https://example.com)\n", 40);
    let has = ls.iter().flatten().any(|c| c.link.is_some());
    assert!(has, "link spans carry their target");
}

#[test]
fn zero_width_never_panics() {
    let _ = lines("# x\n", 0);
}
