//! The gate's whole value is in what it refuses, so most of these are about refusals.
use super::*;
use crate::broker::tier::Tier;

fn gate() -> Gate {
    Gate::new()
}

fn chan() -> Requester {
    Requester::Channel("telegram:me".into())
}

/// Reads and reversible writes never stop to ask, whoever asked for them. An assistant that
/// confirms every file listing is not an assistant.
#[test]
fn anything_that_can_be_undone_just_runs() {
    let mut g = gate();
    for tier in [Tier::Read, Tier::Reversible] {
        for who in [
            Requester::LocalPane,
            chan(),
            Requester::Trigger("7am".into()),
        ] {
            assert_eq!(
                g.decide("t", tier, &who, Policy::default(), 0),
                Decision::Allow,
                "{tier:?} from {who:?} should not need approval"
            );
        }
    }
    assert!(g.pending().is_empty(), "nothing was queued for a human");
}

/// Today's behaviour, preserved deliberately: a person typing into a pane on their own machine
/// is already the approval. Interrupting them to confirm their own keystroke is theatre.
#[test]
fn a_person_at_the_keyboard_is_already_the_approval() {
    let mut g = gate();
    assert_eq!(
        g.decide(
            "sys:run",
            Tier::Irreversible,
            &Requester::LocalPane,
            Policy::default(),
            0
        ),
        Decision::Allow
    );
    assert!(g.pending().is_empty());
}

/// …but that trust is a policy, not a law. Turning it off makes the local pane ask too.
#[test]
fn local_trust_can_be_switched_off() {
    let mut g = gate();
    let strict = Policy {
        trust_present_human: false,
        ..Policy::default()
    };
    match g.decide(
        "sys:run",
        Tier::Irreversible,
        &Requester::LocalPane,
        strict,
        0,
    ) {
        Decision::Ask { reply_to, .. } => assert_eq!(reply_to, "pane"),
        other => panic!("expected an approval request, got {other:?}"),
    }
}

/// A phone has a human at the other end, but they are not watching the tool run.
#[test]
fn a_channel_request_opens_an_approval_addressed_back_to_it() {
    let mut g = gate();
    match g.decide(
        "gmail:send",
        Tier::Irreversible,
        &chan(),
        Policy::default(),
        100,
    ) {
        Decision::Ask { id, reply_to } => {
            assert_eq!(
                reply_to, "telegram:me",
                "the question goes where it came from"
            );
            assert_eq!(g.pending().len(), 1);
            assert_eq!(g.pending()[0].id, id);
            assert_eq!(g.pending()[0].asked_ms, 100);
        }
        other => panic!("expected an approval request, got {other:?}"),
    }
}

/// The 3am case. A trigger has nobody to ask, so the honest answer is no — the alternative is
/// opening a question into an empty room and reading the silence as a yes.
#[test]
fn a_trigger_with_nobody_to_ask_is_denied_not_queued() {
    let mut g = gate();
    match g.decide(
        "bank:pay",
        Tier::Irreversible,
        &Requester::Trigger("nightly".into()),
        Policy::default(),
        0,
    ) {
        Decision::Deny(why) => assert!(
            why.contains("bank:pay"),
            "the refusal names the tool: {why}"
        ),
        other => panic!("expected a denial, got {other:?}"),
    }
    assert!(
        g.pending().is_empty(),
        "nothing is left waiting for a ghost"
    );
}

#[test]
fn granting_returns_the_request_and_clears_it() {
    let mut g = gate();
    let Decision::Ask { id, .. } = g.decide(
        "gmail:send",
        Tier::Irreversible,
        &chan(),
        Policy::default(),
        0,
    ) else {
        panic!("expected an approval request");
    };
    let (p, outcome) = g.answer(&id, true).expect("the id is known");
    assert_eq!(outcome, Outcome::Granted);
    assert_eq!(p.tool, "gmail:send");
    assert_eq!(p.requester, chan());
    assert!(g.pending().is_empty());
}

/// An approval is single-use. Without this, anyone who saw a granted id could replay it and fire
/// the action again.
#[test]
fn an_approval_cannot_be_answered_twice() {
    let mut g = gate();
    let Decision::Ask { id, .. } = g.decide(
        "gmail:send",
        Tier::Irreversible,
        &chan(),
        Policy::default(),
        0,
    ) else {
        panic!("expected an approval request");
    };
    assert!(g.answer(&id, true).is_some());
    assert!(
        g.answer(&id, true).is_none(),
        "a second grant finds nothing to re-fire"
    );
    assert!(g.answer(&id, false).is_none());
}

#[test]
fn answering_an_unknown_id_finds_nothing() {
    let mut g = gate();
    assert!(g.answer("a999", true).is_none());
}

/// Silence is not consent. An approval nobody answered must lapse into a denial, and it must be
/// distinguishable from a deliberate no when someone reads the ledger later.
#[test]
fn an_unanswered_approval_lapses_and_is_reported() {
    let mut g = gate();
    g.decide(
        "gmail:send",
        Tier::Irreversible,
        &chan(),
        Policy::default(),
        1_000,
    );
    let p = Policy::default();
    assert!(
        g.expire(1_000 + p.timeout_ms - 1, p).is_empty(),
        "it is still live one millisecond before the deadline"
    );
    assert_eq!(g.pending().len(), 1);
    let expired = g.expire(1_000 + p.timeout_ms, p);
    assert_eq!(expired.len(), 1, "at the deadline it lapses");
    assert_eq!(expired[0].tool, "gmail:send");
    assert!(g.pending().is_empty());
    assert_ne!(
        Outcome::TimedOut,
        Outcome::Denied,
        "a lapse and a refusal are different facts"
    );
}

/// Expiry must only take the stale ones — a fresh request sitting behind an old one keeps waiting.
#[test]
fn expiry_leaves_the_young_requests_alone() {
    let mut g = gate();
    let p = Policy::default();
    g.decide("old", Tier::Irreversible, &chan(), p, 0);
    g.decide("new", Tier::Irreversible, &chan(), p, p.timeout_ms);
    let expired = g.expire(p.timeout_ms, p);
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].tool, "old");
    assert_eq!(g.pending().len(), 1);
    assert_eq!(g.pending()[0].tool, "new");
}

/// Ids must not repeat: a reused id would let an answer land on the wrong action.
#[test]
fn approval_ids_are_unique_across_answers() {
    let mut g = gate();
    let p = Policy::default();
    let Decision::Ask { id: first, .. } = g.decide("a", Tier::Irreversible, &chan(), p, 0) else {
        panic!()
    };
    g.answer(&first, true);
    let Decision::Ask { id: second, .. } = g.decide("b", Tier::Irreversible, &chan(), p, 0) else {
        panic!()
    };
    assert_ne!(first, second);
}
