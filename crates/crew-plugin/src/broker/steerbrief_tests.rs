//! What the user typed into a run reaches the lead's closing brief, the
//! judge's, and a revision's task (`steer::told`), after the request itself.
use super::super::swarmanswer::prompt;
use super::{judge_prompt, revise_task};
use crate::broker::steer::told_in_test;

const SAID: &str = "also check the tests";

/// This thread's task told [`SAID`] for as long as it lives, and nothing after:
/// tests may share a thread (`--test-threads=1`).
struct Told;

impl Told {
    fn new() -> Self {
        told_in_test(&[SAID]);
        Told
    }
}

impl Drop for Told {
    fn drop(&mut self) {
        told_in_test(&[]);
    }
}

#[test]
fn every_brief_carries_the_steer_after_the_request() {
    let _told = Told::new();
    for brief in [
        prompt("compare the two", &[]),
        judge_prompt("compare the two", "result"),
        revise_task("compare the two", "a gap", ""),
    ] {
        let (request, said) = (brief.find("compare the two"), brief.find(SAID));
        assert!(request.is_some() && request < said, "{brief}");
    }
}

#[test]
fn an_untold_run_briefs_as_it_always_did() {
    for brief in [
        prompt("goal", &[]),
        judge_prompt("goal", "r"),
        revise_task("goal", "g", ""),
    ] {
        assert!(!brief.contains(crew_hive::steers::HEAD), "{brief}");
    }
}
