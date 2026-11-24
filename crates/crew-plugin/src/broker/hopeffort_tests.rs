//! The route's level reaches every model call of a relay hop's turn on the
//! harness — the native loop's and the text loop's — and the next message,
//! routed nowhere special, sends none.
use crew_hive::Effort;

use super::kit::*;
use crate::broker::routeeffort;

fn efforts(t: &Turn) -> Vec<Option<Effort>> {
    t.seen.iter().map(|r| r.effort).collect()
}

#[test]
fn a_routed_hop_asks_every_call_of_its_turn_for_the_route_s_level() {
    let t = {
        let _route = routeeffort::enter(Some(Effort::High));
        go(
            true,
            vec![
                Ok(calling("fs__read", serde_json::json!({"path": "a.rs"}))),
                Ok(said("a.rs holds main\n@done")),
            ],
        )
    };
    assert_eq!(efforts(&t), vec![Some(Effort::High); 2]);
    let text = {
        let _route = routeeffort::enter(Some(Effort::Low));
        go(false, vec![Ok(said("done\n@done"))])
    };
    assert_eq!(efforts(&text), vec![Some(Effort::Low)]);
}

#[test]
fn once_the_routed_message_ends_the_next_hop_sends_no_level() {
    {
        let _route = routeeffort::enter(Some(Effort::High));
    }
    let t = go(true, vec![Ok(said("hi\n@done"))]);
    assert_eq!(efforts(&t), vec![None]);
}

#[test]
fn a_hop_routed_high_on_a_model_that_thinks_gets_the_room_for_it() {
    let ceilings = |id: &str, e: Option<Effort>| {
        let _route = routeeffort::enter(e);
        let t = hop_on(
            id,
            true,
            vec![Ok(said("done\n@done"))],
            4,
            Default::default(),
        );
        t.seen.iter().map(|r| r.max_tokens).collect::<Vec<_>>()
    };
    assert_eq!(
        ceilings("claude-opus-5-5", Some(Effort::High)),
        vec![16_000]
    );
    assert_eq!(ceilings("claude-opus-5-5", None), vec![4096]);
    assert_eq!(ceilings("m", Some(Effort::High)), vec![4096]);
}
