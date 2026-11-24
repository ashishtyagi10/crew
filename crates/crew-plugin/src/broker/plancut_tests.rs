//! A draft that spent its whole ceiling thinking is not a plan: it comes
//! back as the cut-off marker alone (`cutoff::whole` on an empty cut
//! reply), and holding it would have approval run the marker as a swarm.
use super::*;
use crate::broker::testenv;
use crew_hive::provider::cutoff::{marked, CUT_OFF};

#[test]
fn a_draft_cut_off_before_it_said_anything_is_not_held() {
    // What a reply cut before any text reaches the plan arm as.
    assert_eq!(marked("", true), CUT_OFF);
    let _g = testenv::mock_with_specialists(CUT_OFF, testenv::TRIO);
    let mut s = Session::new();
    *lock(&s.plan) = Some(PendingPlan {
        task: "an older draft".into(),
        plan: "1. old".into(),
        verify: false,
    });
    let mut out = Vec::new();
    let mut emit = |ev| {
        out.push(ev);
        Ok(())
    };
    plan_cmd(&mut s, "redesign the scheduler", false, &mut emit).unwrap();
    assert!(lock(&s.plan).is_none(), "nothing is pending");
    assert!(
        !out.iter()
            .any(|e| matches!(e, PluginEvent::Plan { pending: true })),
        "no approval affordance"
    );
    // The older draft's enter/esc goes with it.
    assert!(out
        .iter()
        .any(|e| matches!(e, PluginEvent::Plan { pending: false })));
    let said = out.iter().any(|e| {
        matches!(e, PluginEvent::Message { text, .. } if text.contains("cut off at the token limit before"))
    });
    assert!(said, "the user is told why");
}
