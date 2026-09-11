//! The model's pick outranks the name match; every stop lands on the name
//! match exactly; one decision is one call. The env test holds `testenv`'s
//! lock: `CREW_SKILL_PICK=0` is process-wide.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::broker::skills::parse as parse_skill;
use crate::broker::testenv;

fn skill(name: &str, body: &str) -> Skill {
    parse_skill(body, name, "user")
}

/// Three skills, in the library's (sorted) order.
fn library() -> Vec<Skill> {
    vec![
        skill(
            "deploy",
            "---\ndescription: ship safely\n---\nRun the canary.",
        ),
        skill("review", "Check unsafe blocks first."),
        skill("tests", "# Tests first\nWrite the failing test."),
    ]
}

fn names<'s>(p: &Pick<'s>) -> Vec<&'s str> {
    p.skills.iter().map(|s| s.name.as_str()).collect()
}

/// A decider whose chooser answers `reply` and counts its calls.
fn counting(reply: &'static str, calls: &Arc<AtomicUsize>) -> Decider {
    let calls = Arc::clone(calls);
    Decider::fixed(Box::new(move |_: &str| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(reply.to_string())
    }))
}

#[test]
fn the_chosen_names_are_applied_in_roster_order_and_marked_as_the_models() {
    let lib = library();
    let d = Decider::fixed(Box::new(|_| Ok("SKILLS: tests, deploy".into())));
    let p = d.pick("make it ship", &lib);
    assert_eq!(
        names(&p),
        ["deploy", "tests"],
        "roster order, not reply order"
    );
    assert!(p.by_model);
}

#[test]
fn an_unknown_name_is_dropped_and_the_known_one_kept() {
    let lib = library();
    let d = Decider::fixed(Box::new(|_| Ok("SKILLS: review, nonesuch".into())));
    let p = d.pick("look at lib.rs", &lib);
    assert_eq!(names(&p), ["review"]);
    assert!(p.by_model);
}

/// The model's `none` outranks the substring: a task that merely CONTAINS a
/// skill's name gets nothing when the model says nothing fits.
#[test]
fn none_applies_no_skill_even_when_the_task_mentions_one() {
    let lib = library();
    assert_eq!(
        names(&fallback("review my deploy plan", &lib)),
        ["deploy", "review"]
    );
    let d = Decider::fixed(Box::new(|_| Ok("SKILLS: none".into())));
    let p = d.pick("review my deploy plan", &lib);
    assert!(p.skills.is_empty(), "{:?}", names(&p));
    assert!(p.by_model, "a decision, not a fallback");
}

#[test]
fn a_failed_or_off_grammar_call_falls_back_to_the_name_match_exactly() {
    let lib = library();
    let task = "run a review of lib.rs, then deploy";
    let expect = names(&fallback(task, &lib));
    assert_eq!(expect, ["deploy", "review"]);
    for reply in [
        Err("provider down".to_string()),
        Ok("I'd say review".to_string()),
    ] {
        let d = Decider::fixed(Box::new(move |_| reply.clone()));
        let p = d.pick(task, &lib);
        assert_eq!(names(&p), expect);
        assert!(!p.by_model, "the name match answered");
    }
}

#[test]
fn one_skill_makes_no_call_and_applies_by_name_as_before() {
    let one = vec![skill("review", "Check unsafe blocks first.")];
    let calls = Arc::new(AtomicUsize::new(0));
    let d = counting("SKILLS: none", &calls);
    assert_eq!(names(&d.pick("run a review", &one)), ["review"]);
    assert!(d.pick("say hello", &one).skills.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0, "nothing to choose between");
    assert!(d.pick("say hello", &[]).skills.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn one_decision_is_one_call_for_two_reads_of_the_same_task() {
    let lib = library();
    let calls = Arc::new(AtomicUsize::new(0));
    let d = counting("SKILLS: tests", &calls);
    let first = names(&d.pick("make the suite green", &lib));
    let second = names(&d.pick("make the suite green", &lib));
    assert_eq!(first, ["tests"]);
    assert_eq!(first, second);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "memo answered the second read"
    );
    d.pick("a different task", &lib);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "a new task is a new question"
    );
}

#[test]
fn crew_skill_pick_0_makes_no_call_and_the_name_match_decides() {
    let _env = testenv::mock("unused");
    struct Unset;
    impl Drop for Unset {
        fn drop(&mut self) {
            std::env::remove_var("CREW_SKILL_PICK");
        }
    }
    let _unset = Unset;
    std::env::set_var("CREW_SKILL_PICK", "0");
    let lib = library();
    let calls = Arc::new(AtomicUsize::new(0));
    let d = counting("SKILLS: none", &calls);
    let p = d.pick("run a review", &lib);
    assert_eq!(names(&p), ["review"]);
    assert!(!p.by_model);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(live().is_none(), "and the live chooser is off too");
}

#[test]
fn the_off_decider_never_calls_and_the_mock_has_no_live_chooser() {
    let _env = testenv::mock("unused");
    let lib = library();
    let p = Decider::off().pick("run a review", &lib);
    assert_eq!(names(&p), ["review"]);
    assert!(!p.by_model);
    assert!(live().is_none(), "the mock provider never chooses");
}

#[test]
fn the_grammar_reads_a_skills_first_line_and_nothing_else() {
    let lib = library();
    assert_eq!(parse("SKILLS: review", &lib).unwrap(), ["review"]);
    assert_eq!(
        parse("**skills:** `Review`, `tests`.", &lib).unwrap(),
        ["review", "tests"]
    );
    assert_eq!(
        parse("\n\nSKILLS: none\nbecause", &lib).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        parse("SKILLS: tests, review, deploy", &lib).unwrap(),
        ["review", "tests"],
        "the first two named win, in roster order"
    );
    assert_eq!(parse("SKILLS: review, review", &lib).unwrap(), ["review"]);
    assert!(
        parse("SKILLS: nonesuch", &lib).is_none(),
        "nothing known is no choice"
    );
    assert!(parse("TOOLS: review", &lib).is_none());
    assert!(parse("review", &lib).is_none());
    assert!(parse("", &lib).is_none());
}

#[test]
fn the_prompt_states_the_grammar_and_lists_each_skill_on_one_line() {
    let lib = library();
    let p = prompt("make the suite green", &lib);
    assert!(p.contains("`SKILLS: <name>, <name>`"), "{p}");
    assert!(p.contains("`SKILLS: none`"), "{p}");
    assert!(p.contains("deploy \u{2014} ship safely\n"), "{p}");
    assert!(
        p.contains("review \u{2014} Check unsafe blocks first.\n"),
        "{p}"
    );
    assert!(p.contains("tests \u{2014} Tests first\n"), "{p}");
    assert!(p.ends_with("Task: make the suite green"), "{p}");
    let g = p.find("SKILLS:").unwrap();
    let r = p.find("Skills:\n").unwrap();
    let t = p.find("Task:").unwrap();
    assert!(g < r && r < t, "grammar, roster, task: {p}");
}
