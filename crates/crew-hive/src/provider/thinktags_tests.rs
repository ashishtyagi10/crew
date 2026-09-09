use super::*;

fn t(s: &str) -> Piece {
    Piece::Text(s.into())
}
fn th(s: &str) -> Piece {
    Piece::Thought(s.into())
}

#[test]
fn a_tag_split_across_fragments_still_routes_the_thought() {
    let mut tags = ThinkTags::default();
    let mut got = Vec::new();
    for frag in ["<thi", "nk>foo</th", "ink>bar"] {
        got.extend(tags.feed(frag));
    }
    got.extend(tags.finish());
    assert_eq!(got, vec![th("foo"), t("bar")]);
}

#[test]
fn a_leading_tag_at_the_very_start_emits_no_empty_text() {
    let mut tags = ThinkTags::default();
    assert_eq!(tags.feed("<think>"), Vec::<Piece>::new());
    assert_eq!(tags.feed("hm"), vec![th("hm")]);
    assert_eq!(tags.feed("</think>\n\nanswer"), vec![t("\n\nanswer")]);
}

#[test]
fn text_without_tags_passes_through_untouched_fragment_by_fragment() {
    let mut tags = ThinkTags::default();
    assert_eq!(tags.feed("a < b"), vec![t("a < b")]);
    assert_eq!(tags.feed(" and <t"), vec![t(" and ")], "`<t` is held");
    assert_eq!(
        tags.feed("wo"),
        vec![t("<two")],
        "…and released once it is not a tag"
    );
    assert_eq!(tags.finish(), Vec::<Piece>::new());
}

#[test]
fn an_unclosed_tag_at_the_end_is_released_as_text_on_finish() {
    let mut tags = ThinkTags::default();
    assert_eq!(tags.feed("done <thin"), vec![t("done ")]);
    assert_eq!(tags.finish(), vec![t("<thin")]);
}

#[test]
fn split_takes_a_whole_body() {
    assert_eq!(
        ThinkTags::split("<think>why</think>because"),
        ("because".to_string(), "why".to_string())
    );
    assert_eq!(
        ThinkTags::split("plain"),
        ("plain".to_string(), String::new())
    );
}
