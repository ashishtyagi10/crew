use super::*;

fn ans(pane: &str, text: Option<&str>) -> CastAnswer {
    CastAnswer {
        pane: pane.into(),
        label: None,
        text: text.map(str::to_string),
        no_answer: text.map_or(Some(NoAnswer::IdleNoEngage), |_| None),
    }
}

#[test]
fn any_settles_on_first_answer_with_only_the_winners() {
    let collected = vec![ans("p0", None), ans("p1", Some("42"))];
    let out = settle(CastMode::Any, &collected, false).expect("settled");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].text.as_deref(), Some("42"));
}

#[test]
fn any_waits_while_only_non_answers_and_targets_remain() {
    let collected = vec![ans("p0", None)];
    assert!(settle(CastMode::Any, &collected, false).is_none());
}

#[test]
fn any_returns_the_no_answers_once_exhausted() {
    let collected = vec![ans("p0", None), ans("p1", None)];
    let out = settle(CastMode::Any, &collected, true).expect("exhausted");
    assert_eq!(out.len(), 2, "no winner: report why each stayed silent");
}

#[test]
fn all_waits_until_every_target_resolves() {
    let collected = vec![ans("p0", Some("a"))];
    assert!(settle(CastMode::All, &collected, false).is_none());
    let out = settle(CastMode::All, &collected, true).expect("all in");
    assert_eq!(out.len(), 1);
}
