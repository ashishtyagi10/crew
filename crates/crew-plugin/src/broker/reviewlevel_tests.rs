//! The level comes from the asker's own words, never from a name they cite.
use super::*;

#[test]
fn the_asker_s_words_pick_the_level() {
    for (task, want) in [
        ("give it a quick look", Level::Quick),
        ("glance over my changes", Level::Quick),
        ("skim the diff before I push", Level::Quick),
        ("deep security review of auth", Level::Deep),
        ("a thorough review of my changes", Level::Deep),
        ("review my diff carefully.", Level::Deep),
        ("an in-depth review of the patch", Level::Deep),
        ("review my diff", Level::Standard),
        ("look over my changes", Level::Standard),
        // Both said: the harder look wins.
        ("a quick but thorough review of my diff", Level::Deep),
    ] {
        assert_eq!(Level::of_ask(task), want, "{task}");
    }
}

#[test]
fn file_names_and_code_never_pick_it() {
    for task in [
        "review my changes to quick.rs",
        "review the diff in src/deep/walk.rs",
        "review my edits to `quick look` handling",
        "review my changes to deep_merge",
        "review the diff around fast::path",
        "review my changes to skim()",
        "review my deep-link changes",
    ] {
        assert_eq!(Level::of_ask(task), Level::Standard, "{task}");
    }
}

#[test]
fn quick_asks_for_five_and_reads_no_callers() {
    let p = prompt(Level::Quick, "DIFFBODY", "");
    assert!(p.contains("at most 5 findings"), "{p}");
    assert!(p.contains("DIFFBODY"));
    assert!(!Level::Quick.reads_callers());
    assert!(Level::Deep.reads_callers() && Level::Standard.reads_callers());
    assert_eq!(Level::Quick.effort(), Some(Effort::Low));
    assert_eq!(Level::Deep.effort(), Some(Effort::High));
}

#[test]
fn standard_is_the_prompt_it_always_was() {
    let c = "CALLERS (x)\nfoo — a.rs:1: foo()";
    assert_eq!(
        prompt(Level::Standard, "d", c),
        crate::broker::review::review_prompt("d", c)
    );
    assert_eq!(Level::Standard.effort(), None);
    assert_eq!(Level::Standard.label(3), None);
}

#[test]
fn the_label_names_the_level_and_the_count() {
    assert_eq!(Level::Quick.label(3).unwrap(), "quick look · 3 findings");
    assert_eq!(Level::Deep.label(1).unwrap(), "deep review · 1 finding");
    assert_eq!(Level::Quick.label(0).unwrap(), "quick look · no findings");
}

#[test]
fn words_that_name_the_code_never_pick_it() {
    for task in [
        "review the diff that speeds up the fast path",
        "review my rough draft of the parser changes",
        "review my changes to the brief formatter",
        "review my security fix diff",
        "review my diff to the quick sort",
    ] {
        assert_eq!(Level::of_ask(task), Level::Standard, "{task}");
    }
    assert_eq!(Level::of_ask("a security audit of my diff"), Level::Deep);
    assert_eq!(Level::of_ask("review my diff quickly"), Level::Quick);
}

fn tidied(text: &str) -> Tidied {
    Tidied {
        text: text.into(),
        dropped: vec!["kept note".into()],
    }
}

#[test]
fn a_quick_look_keeps_the_five_worst_and_says_how_many_went() {
    let mut r: Vec<String> = (1..=6)
        .map(|n| format!("- nit — lib.rs:{n} — thing {n}"))
        .collect();
    r.push("- blocker — lib.rs:7 — the one that matters".into());
    r.extend(["".into(), "verdict: 1 blocker, 6 nit".into()]);
    let t = worst(tidied(&r.join("\n")), 5);
    assert_eq!(count(&t.text), 5, "{}", t.text);
    assert!(t.text.contains("lib.rs:7"), "the blocker written 7th stays");
    assert!(t.text.contains("lib.rs:4") && !t.text.contains("lib.rs:5 "));
    assert!(
        t.text
            .ends_with("verdict: 1 blocker, 4 nit\n2 more not shown — ask for a review for all"),
        "{}",
        t.text
    );
    assert_eq!(t.dropped, ["kept note"]);
}

#[test]
fn five_or_fewer_are_left_as_they_were() {
    let text = "- warn — a.rs:1 — w\n\nverdict: 1 warn";
    assert_eq!(worst(tidied(text), 5), tidied(text));
}
