//! The loop, driven by scripted verdicts and turns: no provider, no relay,
//! no shared `turnproof` log for parallel tests to race on.
use std::cell::RefCell;
use std::collections::VecDeque;

use super::*;

const TASK: &str = "rename x to y and make sure the tests pass";
/// A revision that ran something.
const RAN: &str = "1. sys:run {\"cmd\": \"cargo test\"} \u{2192} ok: test result: ok";
/// A revision that ran nothing and answered as the turn before it did.
const SAME: &str = "(same)";

/// What a scripted run did: the briefs the judge read, the task each turn
/// was handed (the first is the request, the rest are revisions), and the
/// pane's events — smith's lines, and `thinking`/`idle` around each check.
struct Ran {
    briefs: Vec<String>,
    turns: Vec<String>,
    events: Vec<String>,
}

impl Ran {
    fn revisions(&self) -> usize {
        self.turns.len() - 1
    }

    fn lines(&self) -> Vec<&str> {
        let chrome = ["thinking", "idle"];
        let lines = self.events.iter().filter(|e| !chrome.contains(&e.as_str()));
        lines.map(String::as_str).collect()
    }
}

/// Run the loop: the judge answers `verdicts` in order, turn `n` reports
/// `proofs[n]` (or [`RAN`]), and `/stop` lands once `stop_at` checks ran.
fn drive(verdicts: &[Result<&str, &str>], proofs: &[&str], stop_at: usize) -> Ran {
    let queue = RefCell::new(verdicts.iter().copied().collect::<VecDeque<_>>());
    let briefs = RefCell::new(Vec::new());
    let judge = |p: &str| {
        briefs.borrow_mut().push(p.to_string());
        let v = queue.borrow_mut().pop_front().unwrap();
        v.map(str::to_string).map_err(str::to_string)
    };
    let mut turns = Vec::new();
    let mut events = Vec::new();
    run(
        TASK,
        &judge,
        &mut |t: &str, _: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>| {
            let n = turns.len();
            turns.push(t.to_string());
            let proof = proofs.get(n).copied().unwrap_or(RAN);
            let answer = format!("answer {}", if proof == SAME { n - 1 } else { n });
            let proof = proof.replace(SAME, "");
            Ok(Some(Turn { answer, proof }))
        },
        &|| briefs.borrow().len() >= stop_at,
        &mut |ev| {
            events.push(match ev {
                PluginEvent::Message { text, .. } => text,
                PluginEvent::Activity { state, .. } => state,
                other => format!("{other:?}"),
            });
            Ok(())
        },
    )
    .unwrap();
    Ran {
        briefs: briefs.into_inner(),
        turns,
        events,
    }
}

#[test]
fn met_the_first_time_is_one_check_and_no_revision() {
    let r = drive(&[Ok("MET: the suite is green")], &[], usize::MAX);
    assert_eq!((r.briefs.len(), r.revisions()), (1, 0));
    let line = "verified \u{2014} the suite is green";
    assert_eq!(r.events, ["thinking", line, "idle"]);
}

/// Each revision is judged on what IT did, and goes back on the newest gap.
#[test]
fn new_gaps_keep_it_going_until_the_judge_says_met() {
    let verdicts = [
        Ok("NOT MET: c.rs still uses x"),
        Ok("NOT MET: the tests were never run"),
        Ok("MET: renamed and green"),
    ];
    let r = drive(&verdicts, &[], usize::MAX);
    assert_eq!((r.briefs.len(), r.revisions()), (3, 2));
    assert!(r.turns[2].starts_with("A check of your last turn found: the tests were never run"));
    assert!(r.briefs[2].contains("answer 2"), "{}", r.briefs[2]);
    let lines = [
        "not yet \u{2014} c.rs still uses x",
        "not yet \u{2014} the tests were never run",
        "verified \u{2014} renamed and green",
    ];
    assert_eq!(r.lines(), lines);
}

#[test]
fn the_same_gap_twice_stops_after_two_checks() {
    let verdicts = [
        Ok("NOT MET: The tests were never run."),
        Ok("NOT MET: the `tests` were never run"),
    ];
    let r = drive(&verdicts, &[], usize::MAX);
    assert_eq!((r.briefs.len(), r.revisions()), (2, 1));
    let stop = "stopped checking \u{2014} no progress: the `tests` were never run";
    assert_eq!(r.lines().last(), Some(&stop));
}

/// Nothing ran and nothing new was said: the judge is not asked again.
#[test]
fn a_revision_that_changed_nothing_stops_without_a_second_check() {
    let r = drive(
        &[Ok("NOT MET: the tests were never run")],
        &[RAN, SAME],
        usize::MAX,
    );
    assert_eq!((r.briefs.len(), r.revisions()), (1, 1));
    let stop = "stopped checking \u{2014} the revision changed nothing: the tests were never run";
    assert_eq!(
        r.lines(),
        ["not yet \u{2014} the tests were never run", stop]
    );
}

/// A gap in the words is closed with words: no tool, a new answer, judged.
#[test]
fn a_revision_in_words_alone_is_still_judged() {
    let verdicts = [Ok("NOT MET: the files are not listed"), Ok("MET: listed")];
    let r = drive(&verdicts, &[RAN, " "], usize::MAX);
    assert_eq!((r.briefs.len(), r.revisions()), (2, 1));
    assert_eq!(r.lines().last(), Some(&"verified \u{2014} listed"));
}

#[test]
fn three_checks_is_the_cap_and_the_last_miss_is_not_sent_back() {
    let verdicts = [Ok("NOT MET: a"), Ok("NOT MET: b"), Ok("NOT MET: c")];
    let r = drive(&verdicts, &[], usize::MAX);
    assert_eq!(
        (r.briefs.len(), r.revisions()),
        (MAX_CHECKS, MAX_CHECKS - 1)
    );
    let stop = "stopped checking \u{2014} 3 checks used: c";
    assert_eq!(r.lines().last(), Some(&stop));
    assert_eq!(r.events.last().map(String::as_str), Some("idle"));
}

#[test]
fn a_judge_that_errors_sends_nothing_back() {
    let r = drive(&[Err("timed out after 60s")], &[], usize::MAX);
    assert_eq!((r.briefs.len(), r.revisions()), (1, 0));
    assert_eq!(r.lines(), ["could not verify: timed out after 60s"]);
    let r = drive(
        &[Ok("NOT MET: a"), Err("provider said 529")],
        &[],
        usize::MAX,
    );
    assert_eq!((r.briefs.len(), r.revisions()), (2, 1));
    assert_eq!(
        r.lines().last(),
        Some(&"could not verify: provider said 529")
    );
}

#[test]
fn a_stop_after_the_verdict_sends_nothing_back() {
    let r = drive(&[Ok("NOT MET: a")], &[], 1);
    assert_eq!((r.briefs.len(), r.revisions()), (1, 0));
    assert_eq!(r.lines(), ["not yet \u{2014} a"]);
}

#[test]
fn a_turn_that_left_no_answer_is_not_judged() {
    let mut events = 0;
    let judge = |_: &str| -> Result<String, String> { panic!("judged a turn with no answer") };
    let mut turn = |_: &str, _: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>| Ok(None);
    run(TASK, &judge, &mut turn, &|| false, &mut |_| {
        events += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(events, 0);
}
