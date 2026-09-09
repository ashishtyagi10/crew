use super::*;

#[test]
fn fragments_of_one_agent_accumulate_in_one_live_buffer_with_its_clock() {
    let mut t = Thoughts::default();
    t.absorb("coder", "weigh", 1_000);
    t.absorb("coder", "ing", 1_500);
    t.absorb("scout", "hm", 1_200);
    assert_eq!(t.live.len(), 2, "one buffer per agent");
    let c = t.live_of("coder").expect("coder is thinking");
    assert_eq!(
        (c.text.as_str(), c.since_ms, c.last_ms),
        ("weighing", 1_000, 1_500)
    );
    assert!(t.settled.is_empty(), "nothing settled yet");
}

#[test]
fn the_reply_settles_the_live_buffer_into_a_block_anchored_to_its_stamp() {
    let mut t = Thoughts::default();
    t.absorb("coder", "a", 1_000);
    t.absorb("coder", "b", 5_200);
    t.settle("coder", "777", 5_300);
    assert!(t.live_of("coder").is_none(), "the buffer moved");
    assert_eq!(t.settled.len(), 1);
    let b = &t.settled[0];
    assert_eq!(b.agent, "coder");
    assert_eq!(b.anchor.as_deref(), Some("777"));
    assert_eq!(b.text, "ab");
    assert_eq!(b.ms, 4_200, "first fragment to last: 4.2 s");
    assert!(!b.expanded, "collapsed until clicked");
    t.settle("coder", "778", 5_400);
    assert_eq!(
        t.settled.len(),
        1,
        "a reply with no thought settles nothing"
    );
}

#[test]
fn abandon_settles_every_orphan_without_an_anchor_and_drop_live_clears_them() {
    let mut t = Thoughts::default();
    t.absorb("coder", "x", 1_000);
    t.absorb("scout", "y", 1_000);
    t.abandon(2_000);
    assert!(t.live.is_empty());
    assert_eq!(t.settled.len(), 2);
    assert!(
        t.settled.iter().all(|b| b.anchor.is_none()),
        "no reply to sit above"
    );

    let mut t = Thoughts::default();
    t.absorb("coder", "x", 1_000);
    t.drop_live();
    assert!(
        t.live.is_empty() && t.settled.is_empty(),
        "a dead broker's thought is gone"
    );
}
