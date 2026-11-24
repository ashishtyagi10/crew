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
fn a_quick_look_keeps_only_the_first_five_findings() {
    let r: Vec<String> = (1..=7)
        .map(|n| format!("- warn — lib.rs:{n} — thing {n}"))
        .chain(["".into(), "verdict: 7 warn".into()])
        .collect();
    let kept = at_most(&r.join("\n"), 5);
    assert_eq!(count(&kept), 5, "{kept}");
    assert!(
        kept.contains("lib.rs:5") && !kept.contains("lib.rs:6"),
        "{kept}"
    );
}
