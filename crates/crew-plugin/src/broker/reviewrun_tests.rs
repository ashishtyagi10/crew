//! What each level costs and sends, against a real repository and a fake
//! reviewer that counts its calls and the effort each one ran at.
use crate::broker::reviewlevel::Level;
use crew_hive::Effort;

#[path = "reviewrun_fixture_tests.rs"]
mod fixture;
use fixture::{ran, TWO};

#[test]
fn a_quick_look_is_one_cheap_call_on_the_diff_alone() {
    let (r, seen) = ran(Level::Quick, &[TWO]);
    assert_eq!(seen.len(), 1);
    let (body, effort) = &seen[0];
    assert!(!body.contains("CALLERS"), "{body}");
    assert!(body.contains("at most 5 findings"), "{body}");
    assert_eq!(*effort, Some(Effort::Low), "over the route's high");
    let t = r.reply.unwrap().unwrap();
    assert!(
        t.text.starts_with("quick look · 2 findings\n"),
        "{}",
        t.text
    );
}

#[test]
fn a_standard_review_is_today_s_one_call_and_text() {
    let (r, seen) = ran(Level::Standard, &[TWO]);
    assert_eq!(seen.len(), 1);
    let (body, effort) = &seen[0];
    assert!(
        body.contains("CALLERS") && body.contains("app.rs:2:"),
        "{body}"
    );
    assert_eq!(*effort, Some(Effort::High), "the route's level stands");
    let t = r.reply.unwrap().unwrap();
    assert_eq!(t.text, TWO, "no label, nothing changed");
    assert!(r.notes.is_empty());
}

#[test]
fn a_deep_review_reads_callers_and_gives_each_finding_a_second_look() {
    let (r, seen) = ran(Level::Deep, &[TWO, "1 keep\n2 drop — b is used on line 2"]);
    assert_eq!(seen.len(), 2, "today's call plus the second look");
    assert!(seen[0].0.contains("CALLERS") && seen[0].0.contains("DEEP REVIEW"));
    assert!(
        seen[1].0.contains("FINDINGS:\n1. - blocker — lib.rs:1"),
        "{}",
        seen[1].0
    );
    assert!(seen.iter().all(|(_, e)| *e == Some(Effort::High)));
    let t = r.reply.unwrap().unwrap();
    assert!(
        t.text.starts_with("deep review · 1 finding\n"),
        "{}",
        t.text
    );
    assert!(t.text.ends_with("verdict: 1 blocker"), "{}", t.text);
    assert!(!t.text.contains("b unused"), "{}", t.text);
    assert_eq!(
        t.dropped,
        ["lib.rs:2 withdrawn on a second look — b is used on line 2"]
    );
}

#[test]
fn a_deep_review_with_nothing_found_makes_no_second_call() {
    let (r, seen) = ran(Level::Deep, &["no findings"]);
    assert_eq!(seen.len(), 1);
    let t = r.reply.unwrap().unwrap();
    assert!(
        t.text.starts_with("deep review · no findings"),
        "{}",
        t.text
    );
}

#[test]
fn a_finding_the_second_look_skipped_is_said_not_re_checked() {
    let (r, _) = ran(Level::Deep, &[TWO, "2 keep"]);
    let t = r.reply.unwrap().unwrap();
    assert!(t.text.starts_with("deep review · 2 findings"), "{}", t.text);
    assert_eq!(
        r.notes,
        ["deep review: 1 not re-checked — the second look did not name them"]
    );
}

#[test]
fn a_second_look_that_does_not_answer_leaves_the_findings_unverified() {
    let (r, seen) = ran(Level::Deep, &[TWO, "sure, all good"]);
    assert_eq!(seen.len(), 2);
    let t = r.reply.unwrap().unwrap();
    assert!(t.text.starts_with("deep review · 2 findings"), "{}", t.text);
    assert_eq!(r.notes.len(), 1);
    assert!(r.notes[0].contains("unverified"), "{:?}", r.notes);
}

#[test]
fn a_quick_look_that_overruns_shows_the_five_worst_true_ones() {
    // Two cite lines past lib.rs's end; a blocker comes last.
    let mut seven: Vec<String> = (1..=6)
        .map(|n| {
            format!(
                "- nit — lib.rs:{} — thing {n}",
                if n <= 2 { 90 + n } else { n }
            )
        })
        .collect();
    seven.push("- blocker — lib.rs:9 — the bug".into());
    seven.push("- nit — lib.rs:10 — last".into());
    let (r, _) = ran(Level::Quick, &[&seven.join("\n")]);
    let t = r.reply.unwrap().unwrap();
    assert!(t.text.starts_with("quick look · 5 findings"), "{}", t.text);
    assert!(t.text.contains("lib.rs:9 — the bug"), "{}", t.text);
    assert!(
        t.text
            .ends_with("verdict: 1 blocker, 4 nit\n1 more not shown — ask for a review for all"),
        "{}",
        t.text
    );
    assert_eq!(
        t.dropped.len(),
        2,
        "the false ones are dropped, not counted"
    );
}
