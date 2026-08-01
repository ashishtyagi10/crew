fn test_pane() -> crate::chat::ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    crate::chat::ChatPane::new(plugin, "crew".into())
}

#[test]
fn turn_total_accumulates_split_and_cost() {
    let mut pane = test_pane();
    pane.absorb_stats(950, String::new(), 0, 0, 900, 50, 129);
    pane.absorb_stats(100, String::new(), 0, 0, 90, 10, 21);
    assert_eq!(pane.tok_in, 990);
    assert_eq!(pane.tok_out, 60);
    assert_eq!(pane.cost_microusd, 150);
    // Per-agent reply stats must NOT double-count into session totals.
    pane.absorb_stats(500, "coder".into(), 800, 400, 450, 50, 60);
    assert_eq!(pane.tok_in, 990);
    assert_eq!(pane.tok_out, 60);
    assert_eq!(pane.cost_microusd, 150);
}

// `absorb_delta` compares `stream_key(&m.sender) == agent`, normalizing the
// stored sender but not the incoming `agent` — so an arrow-form agent name
// (`"coder \u{2192} user"`, as a relay hop would send) must still match a
// card already opened under the bare name `"coder"`, exactly like
// `settle_stream` (which normalizes both sides) already does.
#[test]
fn absorb_delta_normalizes_arrow_form_agent_name_like_settle_stream_does() {
    let mut pane = test_pane();
    pane.absorb_delta("coder".into(), "Hello".into());
    pane.absorb_delta("coder \u{2192} user".into(), ", world".into());
    assert_eq!(
        pane.streaming.len(),
        1,
        "an arrow-form agent name must append to the existing bare-name card, \
         not open a second one"
    );
    assert_eq!(pane.streaming[0].text, "Hello, world");
}

#[test]
fn reply_stat_stashes_usage_for_the_next_message() {
    let mut pane = test_pane();
    pane.absorb_stats(950, "coder".into(), 1_200, 900, 900, 50, 12_000);
    assert_eq!(
        pane.pending_reply_usage,
        Some(("coder".into(), 900, 50, 12_000)),
        "the stash carries the agent the usage belongs to"
    );
}

#[test]
fn zero_usage_reply_stat_stashes_nothing() {
    // Relay's dangling segment and fan's error path both close a hop with an
    // all-zero stat; CLI-backed agents report none at all. None of those may
    // hang a trailer on the next settled message.
    let mut pane = test_pane();
    pane.absorb_stats(0, "coder".into(), 1_200, 0, 0, 0, 0);
    assert_eq!(pane.pending_reply_usage, None);
}

#[test]
fn turn_level_stat_never_touches_the_reply_stash() {
    let mut pane = test_pane();
    pane.absorb_stats(950, String::new(), 0, 0, 900, 50, 12_000);
    assert_eq!(pane.pending_reply_usage, None);
}

#[test]
fn streaming_cards_carry_no_usage() {
    // Usage attaches only to settled messages — a provisional card opened
    // while a reply stat is stashed must not pick it up.
    let mut pane = test_pane();
    pane.absorb_stats(950, "coder".into(), 1_200, 900, 900, 50, 12_000);
    pane.absorb_delta("coder".into(), "Hel".into());
    assert_eq!(pane.streaming[0].usage, None);
}
